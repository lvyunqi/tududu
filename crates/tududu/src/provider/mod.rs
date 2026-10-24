//! 模型渠道商与接入密钥的管理。

pub mod crypto;
pub mod store;
pub mod upstream;

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

const MAX_TIMEOUT_MS: i32 = 600_000;

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub code: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<i32>,
    #[serde(default)]
    pub max_concurrency: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<i32>,
    #[serde(default)]
    pub max_concurrency: Option<i32>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub api_key: String,
    #[serde(default)]
    pub weight: Option<i32>,
    #[serde(default)]
    pub priority: Option<i16>,
    #[serde(default)]
    pub max_concurrency: Option<i32>,
}

pub async fn create_provider(
    db: &PgPool,
    req: CreateProviderRequest,
) -> Result<store::ProviderRow, ApiError> {
    let code = req.code.trim().to_lowercase();
    if !is_valid_code(&code) {
        return Err(ApiError::bad_request("渠道商标识只能用 2 到 32 位小写字母、数字、短横线或下划线"));
    }

    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("渠道商名称不能为空"));
    }

    let base_url = normalize_base_url(&req.base_url)?;
    let protocol = req.protocol.unwrap_or_else(|| "openai_images".to_owned());
    let timeout_ms = clamp_timeout(req.timeout_ms);
    let max_concurrency = req.max_concurrency.unwrap_or(8).clamp(1, 256);

    let created = store::NewProvider {
        code: &code,
        name,
        protocol: &protocol,
        base_url: &base_url,
        timeout_ms,
        max_concurrency,
    };

    match store::insert_provider(db, &created).await {
        Ok(row) => Ok(row),
        Err(err) if store::is_unique_violation(&err) => {
            Err(ApiError::conflict(40001, "该渠道商标识已存在"))
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn list_providers(db: &PgPool) -> Result<Vec<store::ProviderRow>, ApiError> {
    Ok(store::list_providers(db).await?)
}

pub async fn update_provider(
    db: &PgPool,
    id: Uuid,
    req: UpdateProviderRequest,
) -> Result<store::ProviderRow, ApiError> {
    let base_url = match req.base_url.as_deref() {
        Some(raw) => Some(normalize_base_url(raw)?),
        None => None,
    };

    let timeout_ms = req.timeout_ms.map(|value| clamp_timeout(Some(value)));
    let max_concurrency = req.max_concurrency.map(|value| value.clamp(1, 256));
    let name = req.name.as_deref().map(str::trim);

    let updated = store::update_provider(
        db,
        id,
        name,
        base_url.as_deref(),
        timeout_ms,
        max_concurrency,
        req.enabled,
    )
    .await?;

    updated.ok_or_else(|| ApiError::not_found("渠道商不存在"))
}

pub async fn delete_provider(db: &PgPool, id: Uuid) -> Result<(), ApiError> {
    if store::delete_provider(db, id).await? {
        return Ok(());
    }

    Err(ApiError::not_found("渠道商不存在"))
}

pub async fn create_channel(
    db: &PgPool,
    key: &[u8; 32],
    provider_id: Uuid,
    req: CreateChannelRequest,
) -> Result<store::ChannelRow, ApiError> {
    if store::find_provider(db, provider_id).await?.is_none() {
        return Err(ApiError::not_found("渠道商不存在"));
    }

    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("渠道名称不能为空"));
    }

    let api_key = req.api_key.trim();
    if api_key.len() < 8 {
        return Err(ApiError::bad_request("密钥长度看起来不对"));
    }

    let cipher = crypto::encrypt(key, api_key)
        .map_err(|err| {
            tracing::error!(error = %err, "加密渠道密钥失败");
            ApiError::internal("保存密钥失败")
        })?;

    let created = store::NewChannel {
        provider_id,
        name,
        api_key_cipher: &cipher,
        api_key_hint: &crypto::mask(api_key),
        weight: req.weight.unwrap_or(100).clamp(1, 10_000),
        priority: req.priority.unwrap_or(1).clamp(1, 3),
        max_concurrency: req.max_concurrency.unwrap_or(4).clamp(1, 256),
    };

    Ok(store::insert_channel(db, &created).await?)
}

pub async fn list_channels(
    db: &PgPool,
    provider_id: Uuid,
) -> Result<Vec<store::ChannelRow>, ApiError> {
    if store::find_provider(db, provider_id).await?.is_none() {
        return Err(ApiError::not_found("渠道商不存在"));
    }

    Ok(store::list_channels(db, provider_id).await?)
}

/// 用该渠道商下任意一条可用密钥去拉模型列表，拉回来后整体入库。
pub async fn refresh_models(
    db: &PgPool,
    http: &reqwest::Client,
    key: &[u8; 32],
    provider_id: Uuid,
) -> Result<Vec<store::ModelRow>, ApiError> {
    let provider = store::find_provider(db, provider_id)
        .await?
        .ok_or_else(|| ApiError::not_found("渠道商不存在"))?;

    let channels = store::list_channels(db, provider_id).await?;
    let channel = channels
        .iter()
        .find(|item| item.enabled)
        .ok_or_else(|| ApiError::bad_request("该渠道商还没有启用中的密钥，先添加一条"))?;

    let cipher = store::find_channel_secret(db, channel.id)
        .await?
        .ok_or_else(|| ApiError::internal("读取密钥失败"))?;

    let api_key = crypto::decrypt(key, &cipher).map_err(|err| {
        tracing::error!(error = %err, "解密渠道密钥失败");
        ApiError::internal("读取密钥失败，可能主密钥已更换")
    })?;

    let codes = upstream::list_models(http, &provider.base_url, &api_key, provider.timeout_ms)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, provider = %provider.code, "拉取模型列表失败");
            ApiError::new(axum::http::StatusCode::BAD_GATEWAY, 40002, err)
        })?;

    Ok(store::upsert_models(db, provider_id, &codes).await?)
}

pub async fn list_models(
    db: &PgPool,
    provider_id: Uuid,
) -> Result<Vec<store::ModelRow>, ApiError> {
    if store::find_provider(db, provider_id).await?.is_none() {
        return Err(ApiError::not_found("渠道商不存在"));
    }

    Ok(store::list_models(db, provider_id).await?)
}

fn is_valid_code(code: &str) -> bool {
    let len_ok = (2..=32).contains(&code.chars().count());
    let chars_ok = code
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');

    len_ok && chars_ok
}

fn normalize_base_url(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(ApiError::bad_request("接口地址需要以 http:// 或 https:// 开头"));
    }

    Ok(trimmed.to_owned())
}

fn clamp_timeout(value: Option<i32>) -> i32 {
    value.unwrap_or(180_000).clamp(1_000, MAX_TIMEOUT_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_accepts_lowercase_and_digits() {
        assert!(is_valid_code("openai"));
        assert!(is_valid_code("relay-01_a"));
        assert!(!is_valid_code("OpenAI"));
        assert!(!is_valid_code("a"));
        assert!(!is_valid_code("has space"));
    }

    #[test]
    fn base_url_requires_scheme_and_drops_trailing_slash() {
        assert_eq!(
            normalize_base_url("https://api.example.com/v1/").as_deref(),
            Ok("https://api.example.com/v1")
        );
        assert!(normalize_base_url("api.example.com").is_err());
    }

    #[test]
    fn timeout_is_clamped() {
        assert_eq!(clamp_timeout(None), 180_000);
        assert_eq!(clamp_timeout(Some(10)), 1_000);
        assert_eq!(clamp_timeout(Some(99_999_999)), MAX_TIMEOUT_MS);
    }
}
