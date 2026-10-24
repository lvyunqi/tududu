//! 图像任务：提交、执行、重试。

pub mod runner;
pub mod store;

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

const MAX_PROMPT_CHARS: usize = 2000;
const DEFAULT_MAX_ATTEMPTS: i32 = 3;

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub prompt: String,
    pub model: String,
    #[serde(default)]
    pub provider_id: Option<Uuid>,
    #[serde(default)]
    pub input_asset_id: Option<Uuid>,
}

pub async fn submit(
    db: &PgPool,
    user_id: Uuid,
    req: CreateJobRequest,
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

    if let Some(asset_id) = req.input_asset_id {
        let owned = crate::storage::store::find_owned(db, asset_id, user_id).await?;
        if owned.is_none() {
            return Err(ApiError::not_found("输入图片不存在"));
        }
    }

    let new_job = store::NewJob {
        user_id,
        provider_id: req.provider_id,
        model,
        prompt,
        input_asset_id: req.input_asset_id,
        max_attempts: DEFAULT_MAX_ATTEMPTS,
    };

    Ok(store::insert(db, &new_job).await?)
}

pub async fn get(
    db: &PgPool,
    user_id: Uuid,
    job_id: Uuid,
) -> Result<store::JobRow, ApiError> {
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
