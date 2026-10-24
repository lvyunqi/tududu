use std::time::Instant;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::job::store;
use crate::provider::crypto;
use crate::storage;
use crate::storage::local_store::LocalStore;

/// 任务失败时给用户看的错误码。
pub const ERR_NO_CHANNEL: i32 = 30004;
pub const ERR_UPSTREAM: i32 = 30002;
pub const ERR_INPUT_MISSING: i32 = 30005;

#[derive(Debug, sqlx::FromRow)]
pub struct CallTarget {
    pub channel_id: Uuid,
    pub provider_id: Uuid,
    pub base_url: String,
    pub api_key_cipher: String,
    pub timeout_ms: i32,
}

/// 按优先级取可用的渠道。健康度不是 1 的直接跳过。
pub async fn list_targets(db: &PgPool, provider_id: Option<Uuid>) -> sqlx::Result<Vec<CallTarget>> {
    sqlx::query_as::<_, CallTarget>(
        "SELECT c.id AS channel_id, p.id AS provider_id, p.base_url, c.api_key_cipher, p.timeout_ms \
           FROM channels c JOIN providers p ON p.id = c.provider_id \
          WHERE c.enabled AND p.enabled AND c.health = 1 \
            AND ($1::uuid IS NULL OR p.id = $1) \
          ORDER BY c.priority, c.weight DESC, c.created_at",
    )
    .bind(provider_id)
    .fetch_all(db)
    .await
}

struct CallError {
    retryable: bool,
    http_status: Option<i32>,
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ImageResponse {
    #[serde(default)]
    data: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
struct ImageItem {
    b64_json: Option<String>,
    url: Option<String>,
}

/// 跑一个任务：选渠道、调用上游、落盘，失败按次数重试。
pub async fn execute(
    db: &PgPool,
    files: &LocalStore,
    http: &reqwest::Client,
    key: &[u8; 32],
    job_id: Uuid,
) -> Result<(), ApiError> {
    let job = store::find(db, job_id)
        .await?
        .ok_or_else(|| ApiError::not_found("任务不存在"))?;

    store::mark_running(db, job_id).await?;

    let input_bytes = match job.input_asset_id {
        Some(asset_id) => {
            let asset = crate::storage::store::find_owned(db, asset_id, job.user_id).await?;
            match asset {
                Some(asset) => match files.read(&asset.object_key).await {
                    Ok(bytes) => Some(bytes),
                    Err(err) => {
                        tracing::error!(error = %err, %asset_id, "读取输入图片失败");
                        store::mark_failed(db, job_id, ERR_INPUT_MISSING, "输入图片已丢失").await?;
                        return Ok(());
                    }
                },
                None => {
                    store::mark_failed(db, job_id, ERR_INPUT_MISSING, "输入图片不存在").await?;
                    return Ok(());
                }
            }
        }
        None => None,
    };

    let targets = list_targets(db, job.provider_id).await?;
    if targets.is_empty() {
        store::mark_failed(db, job_id, ERR_NO_CHANNEL, "当前没有可用的模型渠道").await?;
        return Ok(());
    }

    let max_attempts = job.max_attempts.max(1);
    let mut last_message = String::from("上游调用失败");

    for attempt in 1..=max_attempts {
        let target = &targets[(attempt as usize - 1) % targets.len()];
        store::bump_attempt(db, job_id, attempt, target.channel_id).await?;

        let started = Instant::now();
        let outcome = call_once(http, key, target, &job, input_bytes.as_deref()).await;
        let latency_ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);

        match outcome {
            Ok(bytes) => {
                record(
                    db,
                    job_id,
                    attempt,
                    target.channel_id,
                    store::ATTEMPT_OK,
                    Some(200),
                    latency_ms,
                    None,
                    None,
                )
                .await?;
                let asset = storage::save_generated(db, files, job.user_id, &bytes).await?;
                store::mark_succeeded(db, job_id, asset.id).await?;
                return Ok(());
            }
            Err(err) => {
                let status = if err.retryable {
                    store::ATTEMPT_RETRYABLE
                } else {
                    store::ATTEMPT_FATAL
                };
                let body = truncate(&err.message, 2000);
                record(
                    db,
                    job_id,
                    attempt,
                    target.channel_id,
                    status,
                    err.http_status,
                    latency_ms,
                    Some(err.code.as_str()),
                    Some(body.as_str()),
                )
                .await?;

                last_message = err.message;
                if !err.retryable {
                    break;
                }

                if attempt < max_attempts {
                    let backoff = (attempt as u64) * 1000;
                    tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                }
            }
        }
    }

    store::mark_failed(db, job_id, ERR_UPSTREAM, &truncate(&last_message, 300)).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn record(
    db: &PgPool,
    job_id: Uuid,
    attempt_no: i32,
    channel_id: Uuid,
    status: i16,
    http_status: Option<i32>,
    latency_ms: i32,
    error_code: Option<&str>,
    error_body: Option<&str>,
) -> Result<(), ApiError> {
    let item = store::NewAttempt {
        job_id,
        attempt_no,
        channel_id,
        status,
        http_status,
        latency_ms,
        error_code,
        error_body,
    };

    store::insert_attempt(db, &item).await?;
    Ok(())
}

async fn call_once(
    http: &reqwest::Client,
    key: &[u8; 32],
    target: &CallTarget,
    job: &store::JobRow,
    input: Option<&[u8]>,
) -> Result<Vec<u8>, CallError> {
    let api_key = crypto::decrypt(key, &target.api_key_cipher).map_err(|err| CallError {
        retryable: false,
        http_status: None,
        code: "decrypt_failed".to_owned(),
        message: err,
    })?;

    let timeout = std::time::Duration::from_millis(u64::from(target.timeout_ms.max(1000) as u32));
    let base = target.base_url.trim_end_matches('/');

    let request = match input {
        Some(bytes) => {
            let part = reqwest::multipart::Part::bytes(bytes.to_vec())
                .file_name("image.png")
                .mime_str("image/png")
                .map_err(|err| CallError {
                    retryable: false,
                    http_status: None,
                    code: "build_request".to_owned(),
                    message: err.to_string(),
                })?;

            let form = reqwest::multipart::Form::new()
                .part("image", part)
                .text("prompt", job.prompt.clone())
                .text("model", job.model.clone())
                .text("n", "1");

            http.post(format!("{base}/images/edits")).multipart(form)
        }
        None => http
            .post(format!("{base}/images/generations"))
            .json(&serde_json::json!({
                "prompt": job.prompt.clone(),
                "model": job.model.clone(),
                "n": 1
            })),
    };

    let response = request
        .bearer_auth(&api_key)
        .timeout(timeout)
        .send()
        .await
        .map_err(|err| CallError {
            retryable: true,
            http_status: None,
            code: "transport".to_owned(),
            message: format!("请求上游失败：{err}"),
        })?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        let retryable = status.as_u16() >= 500 || status.as_u16() == 429;
        return Err(CallError {
            retryable,
            http_status: Some(i32::from(status.as_u16())),
            code: format!("http_{}", status.as_u16()),
            message: format!("上游返回 {status}：{}", truncate(&body, 500)),
        });
    }

    let parsed: ImageResponse = serde_json::from_str(&body).map_err(|err| CallError {
        retryable: true,
        http_status: Some(i32::from(status.as_u16())),
        code: "parse".to_owned(),
        message: format!("解析上游响应失败：{err}"),
    })?;

    let item = parsed.data.into_iter().next().ok_or_else(|| CallError {
        retryable: true,
        http_status: Some(i32::from(status.as_u16())),
        code: "empty".to_owned(),
        message: "上游没有返回图片".to_owned(),
    })?;

    if let Some(encoded) = item.b64_json {
        return STANDARD
            .decode(encoded.as_bytes())
            .map_err(|err| CallError {
                retryable: true,
                http_status: None,
                code: "decode".to_owned(),
                message: format!("解码图片失败：{err}"),
            });
    }

    if let Some(url) = item.url {
        let bytes = http
            .get(&url)
            .timeout(timeout)
            .send()
            .await
            .and_then(|resp| resp.error_for_status())
            .map_err(|err| CallError {
                retryable: true,
                http_status: None,
                code: "download".to_owned(),
                message: format!("下载结果图失败：{err}"),
            })?
            .bytes()
            .await
            .map_err(|err| CallError {
                retryable: true,
                http_status: None,
                code: "download".to_owned(),
                message: format!("读取结果图失败：{err}"),
            })?;

        return Ok(bytes.to_vec());
    }

    Err(CallError {
        retryable: true,
        http_status: None,
        code: "empty".to_owned(),
        message: "上游响应里既没有图片数据也没有地址".to_owned(),
    })
}

fn truncate(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_owned();
    }

    let kept: String = trimmed.chars().take(limit).collect();
    format!("{kept}…")
}
