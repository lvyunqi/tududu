//! 创作会话：把单轮任务组织成带图的连续对话。

pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::job::{self, CreateJobRequest, JobLink};

const MAX_TITLE_CHARS: usize = 64;
const DEFAULT_TITLE: &str = "新会话";

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub model: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub text: String,
    #[serde(default)]
    pub input_asset_id: Option<Uuid>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub provider_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub session: store::SessionRow,
    pub messages: Vec<store::MessageRow>,
}

#[derive(Debug, Serialize)]
pub struct TurnResult {
    pub user_message_id: Uuid,
    pub assistant_message_id: Uuid,
    pub job_id: Uuid,
}

pub async fn create(
    db: &PgPool,
    user_id: Uuid,
    req: CreateSessionRequest,
) -> Result<store::SessionRow, ApiError> {
    let model = req.model.trim();
    if model.is_empty() {
        return Err(ApiError::bad_request("模型不能为空"));
    }

    let title = sanitize_title(req.title.as_deref());
    Ok(store::insert(db, user_id, &title, model).await?)
}

pub async fn detail(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<SessionDetail, ApiError> {
    let session = store::find_owned(db, session_id, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("会话不存在"))?;

    let messages = store::list_messages(db, session_id, 200).await?;

    Ok(SessionDetail { session, messages })
}

pub async fn rename(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    title: &str,
) -> Result<(), ApiError> {
    let cleaned = sanitize_title(Some(title));
    if cleaned == DEFAULT_TITLE && title.trim().is_empty() {
        return Err(ApiError::bad_request("标题不能为空"));
    }

    if store::rename(db, session_id, user_id, &cleaned).await? {
        return Ok(());
    }

    Err(ApiError::not_found("会话不存在"))
}

/// 提交一轮修改：写两条消息，再建一个任务，任务结束时由执行器回写结果。
pub async fn send_message(
    db: &PgPool,
    user_id: Uuid,
    session_id: Uuid,
    req: SendMessageRequest,
) -> Result<TurnResult, ApiError> {
    let session = store::find_owned(db, session_id, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("会话不存在"))?;

    let text = req.text.trim();
    if text.is_empty() {
        return Err(ApiError::bad_request("提示词不能为空"));
    }

    let user_message = store::NewMessage {
        session_id,
        role: "user",
        content: text,
        input_asset_id: req.input_asset_id,
        status: store::MSG_DONE,
    };
    let user_message = store::insert_message(db, &user_message).await?;

    let assistant = store::NewMessage {
        session_id,
        role: "assistant",
        content: "",
        input_asset_id: None,
        status: store::MSG_PENDING,
    };
    let assistant = store::insert_message(db, &assistant).await?;

    let job_request = CreateJobRequest {
        prompt: text.to_owned(),
        model: session.model.clone(),
        provider_id: req.provider_id,
        input_asset_id: req.input_asset_id,
        size: req.size,
        quality: req.quality,
    };

    let link = JobLink {
        session_id: Some(session_id),
        message_id: Some(assistant.id),
    };

    let created = job::submit(db, user_id, job_request, link).await?;

    store::attach_job(db, assistant.id, created.id).await?;
    store::touch(db, session_id).await?;

    Ok(TurnResult {
        user_message_id: user_message.id,
        assistant_message_id: assistant.id,
        job_id: created.id,
    })
}

fn sanitize_title(raw: Option<&str>) -> String {
    let trimmed = raw.unwrap_or_default().trim();
    if trimmed.is_empty() {
        return DEFAULT_TITLE.to_owned();
    }

    trimmed.chars().take(MAX_TITLE_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_title_falls_back_to_default() {
        assert_eq!(sanitize_title(None), DEFAULT_TITLE);
        assert_eq!(sanitize_title(Some("   ")), DEFAULT_TITLE);
        assert_eq!(sanitize_title(Some("  换背景 ")), "换背景");
    }

    #[test]
    fn long_title_is_truncated() {
        let long = "字".repeat(200);
        assert_eq!(sanitize_title(Some(&long)).chars().count(), MAX_TITLE_CHARS);
    }

    #[test]
    fn detail_types_are_serializable() {
        let detail = SessionDetail {
            session: store::SessionRow {
                id: Uuid::nil(),
                title: "t".to_owned(),
                model: "m".to_owned(),
                status: 1,
                last_active_at: Utc::now(),
                created_at: Utc::now(),
            },
            messages: Vec::new(),
        };

        assert!(serde_json::to_string(&detail).is_ok());
        let _: Option<DateTime<Utc>> = None;
    }
}
