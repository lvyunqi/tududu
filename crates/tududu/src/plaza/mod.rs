//! 提示词广场：发布、浏览、点赞。

pub mod store;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

const MAX_TITLE_CHARS: usize = 64;
const MAX_PROMPT_CHARS: usize = 2000;
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub title: String,
    pub prompt_text: String,
    pub model: String,
    #[serde(default)]
    pub category_id: Option<i64>,
    /// 自定义分类名。填了就按名字找，找不到就新建一个。
    #[serde(default)]
    pub category_name: Option<String>,
    #[serde(default)]
    pub cover_asset_id: Option<Uuid>,
    #[serde(default)]
    pub source_session_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct LikeResult {
    pub liked: bool,
    pub like_count: i32,
}

pub async fn publish(
    db: &PgPool,
    user_id: Uuid,
    req: PublishRequest,
) -> Result<store::PromptRow, ApiError> {
    let title = req.title.trim();
    if title.is_empty() || title.chars().count() > MAX_TITLE_CHARS {
        return Err(ApiError::bad_request("标题需要 1 到 64 个字"));
    }

    let text = req.prompt_text.trim();
    if text.is_empty() || text.chars().count() > MAX_PROMPT_CHARS {
        return Err(ApiError::bad_request("提示词需要 1 到 2000 个字"));
    }

    let model = req.model.trim();
    if model.is_empty() {
        return Err(ApiError::bad_request("模型不能为空"));
    }

    if let Some(asset_id) = req.cover_asset_id {
        let owned = crate::storage::store::find_owned(db, asset_id, user_id).await?;
        if owned.is_none() {
            return Err(ApiError::not_found("封面图不存在"));
        }
    }

    // 命中敏感词就转人工审核，不直接上架
    let hits = store::sensitive_hits(db, &format!("{title}\n{text}")).await?;
    let status = if hits.is_empty() {
        store::STATUS_PUBLISHED
    } else {
        tracing::info!(words = ?hits, "作品命中敏感词，转人工审核");
        store::STATUS_REVIEWING
    };

    // 自定义分类优先：填了名字就按名字找或建，否则用选中的 id
    let category_id = match req
        .category_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(name) => Some(store::ensure_category(db, name).await?),
        None => req.category_id,
    };

    let new_prompt = store::NewPrompt {
        short_code: &new_short_code(),
        author_user_id: user_id,
        title,
        prompt_text: text,
        model,
        category_id,
        cover_asset_id: req.cover_asset_id,
        source_session_id: req.source_session_id,
        status,
    };

    Ok(store::insert(db, &new_prompt).await?)
}

pub async fn detail(db: &PgPool, code: &str) -> Result<store::PromptRow, ApiError> {
    let found = store::find_published(db, code)
        .await?
        .ok_or_else(|| ApiError::not_found("作品不存在或已下架"))?;

    // 浏览量只是展示用，失败不影响返回
    if let Err(err) = store::bump_view(db, found.id).await {
        tracing::warn!(error = %err, "累加浏览量失败");
    }

    Ok(found)
}

pub async fn toggle_like(db: &PgPool, user_id: Uuid, code: &str) -> Result<LikeResult, ApiError> {
    let prompt = store::find_published(db, code)
        .await?
        .ok_or_else(|| ApiError::not_found("作品不存在或已下架"))?;

    let liked = store::toggle_like(db, user_id, prompt.id).await?;
    let delta = if liked { 1 } else { -1 };
    let like_count = store::adjust_like_count(db, prompt.id, delta).await?;

    Ok(LikeResult { liked, like_count })
}

pub async fn unpublish(db: &PgPool, user_id: Uuid, code: &str) -> Result<(), ApiError> {
    if store::hide(db, code, user_id).await? {
        return Ok(());
    }

    Err(ApiError::not_found("作品不存在或已下架"))
}

fn new_short_code() -> String {
    let raw = Uuid::new_v4().into_bytes();
    raw.iter()
        .take(8)
        .map(|byte| CODE_ALPHABET[*byte as usize % CODE_ALPHABET.len()] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_code_is_eight_chars() {
        let code = new_short_code();
        assert_eq!(code.chars().count(), 8);
        assert!(code.chars().all(|c| CODE_ALPHABET.contains(&(c as u8))));
    }
}
