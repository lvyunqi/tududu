//! 图像任务：提交、执行、重试。

pub mod runner;
pub mod store;

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

const MAX_PROMPT_CHARS: usize = 2000;
const DEFAULT_MAX_ATTEMPTS: i32 = 3;
/// 一次最多带几张参考图。再多上游也吃不下，而且计费会失控。
pub const MAX_INPUT_IMAGES: usize = 4;

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub prompt: String,
    pub model: String,
    #[serde(default)]
    pub provider_id: Option<Uuid>,
    #[serde(default)]
    pub input_asset_id: Option<Uuid>,
    /// 多张参考图；有它就用它，input_asset_id 只作单图兼容
    #[serde(default)]
    pub input_asset_ids: Vec<Uuid>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
}

/// 任务与会话的关联。直接提交任务时不带，走会话时带上。
#[derive(Debug, Default, Clone, Copy)]
pub struct JobLink {
    pub session_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
}

pub async fn submit(
    db: &PgPool,
    user_id: Uuid,
    req: CreateJobRequest,
    link: JobLink,
) -> Result<store::JobRow, ApiError> {
    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError::bad_request("提示词不能为空"));
    }

    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(ApiError::bad_request("提示词最多 2000 字"));
    }

    let model = req.model.trim();
    if model.is_empty() {
        return Err(ApiError::bad_request("模型不能为空"));
    }

    // 新字段优先，老的单图字段继续兼容
    let mut inputs = req.input_asset_ids.clone();
    if inputs.is_empty()
        && let Some(single) = req.input_asset_id
    {
        inputs.push(single);
    }

    // 去重但保留顺序
    let mut seen = Vec::with_capacity(inputs.len());
    for id in inputs {
        if !seen.contains(&id) {
            seen.push(id);
        }
    }
    let inputs = seen;

    if inputs.len() > MAX_INPUT_IMAGES {
        return Err(ApiError::bad_request(format!(
            "一次最多带 {MAX_INPUT_IMAGES} 张参考图"
        )));
    }

    for asset_id in &inputs {
        let owned = crate::storage::store::find_owned(db, *asset_id, user_id).await?;
        if owned.is_none() {
            return Err(ApiError::not_found("输入图片不存在"));
        }
    }

    let price =
        crate::billing::price_for(db, model, req.size.as_deref(), req.quality.as_deref()).await?;

    let new_job = store::NewJob {
        user_id,
        provider_id: req.provider_id,
        model,
        prompt,
        input_asset_id: inputs.first().copied(),
        input_asset_ids: &inputs,
        session_id: link.session_id,
        message_id: link.message_id,
        max_attempts: DEFAULT_MAX_ATTEMPTS,
        price,
    };

    let created = store::insert(db, &new_job).await?;

    // 先扣费再执行，失败原路退回，避免跑完才发现余额不够
    if let Err(err) = crate::billing::charge(
        db,
        user_id,
        price,
        Some("job"),
        Some(created.id),
        "提交修图任务",
    )
    .await
    {
        let _ = store::mark_failed(db, created.id, 40201, "余额不足").await;
        return Err(err);
    }

    Ok(created)
}

pub async fn get(db: &PgPool, user_id: Uuid, job_id: Uuid) -> Result<store::JobRow, ApiError> {
    let job = store::find_owned(db, job_id, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("任务不存在"))?;

    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_limit_is_two_thousand_chars() {
        assert_eq!(MAX_PROMPT_CHARS, 2000);
    }
}
