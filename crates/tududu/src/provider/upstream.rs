//! 与上游渠道商的轻量交互，目前只有拉取模型列表。

use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ModelsEnvelope {
    #[serde(default)]
    data: Vec<RemoteModel>,
    #[serde(default)]
    models: Vec<RemoteModel>,
}

#[derive(Debug, Deserialize)]
struct RemoteModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

/// 请求 `{base_url}/models`，把两边认得的字段都收下来。
///
/// 不同中转服务返回结构差异很大，这里只取模型标识，拿不到就跳过，不因此整体失败。
pub async fn list_models(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    timeout_ms: i32,
) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));

    let response = client
        .get(&url)
        .bearer_auth(api_key)
        .timeout(Duration::from_millis(
            u64::from(timeout_ms.max(1000) as u32),
        ))
        .send()
        .await
        .map_err(|err| format!("请求上游失败：{err}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("读取上游响应失败：{err}"))?;

    if !status.is_success() {
        return Err(format!("上游返回 {status}：{}", truncate(&body, 300)));
    }

    let envelope: ModelsEnvelope =
        serde_json::from_str(&body).map_err(|err| format!("解析模型列表失败：{err}"))?;

    let mut models: Vec<String> = Vec::new();
    for item in envelope.data.into_iter().chain(envelope.models) {
        let code = if item.id.is_empty() {
            item.name
        } else {
            item.id
        };
        let code = code.trim().to_owned();
        if !code.is_empty() && !models.contains(&code) {
            models.push(code);
        }
    }

    if models.is_empty() {
        return Err("上游没有返回任何模型".to_owned());
    }

    models.sort();
    Ok(models)
}

fn truncate(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_owned();
    }

    let kept: String = trimmed.chars().take(limit).collect();
    format!("{kept}…")
}
