use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiBody, ApiError};
use crate::http::auth::require_user;
use crate::http::routes::AppState;
use crate::job;
use crate::session::{self, CreateSessionRequest, RenameRequest, SendMessageRequest};

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<ApiBody<session::store::SessionRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let created = session::create(&state.db, current.user_id, req).await?;
    Ok(Json(ApiBody::ok(created)))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiBody<Vec<session::store::SessionRow>>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let rows = session::store::list(&state.db, current.user_id, query.before, limit).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<session::SessionDetail>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let found = session::detail(&state.db, current.user_id, id).await?;
    Ok(Json(ApiBody::ok(found)))
}

pub async fn rename(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameRequest>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    session::rename(&state.db, current.user_id, id, &req.title).await?;

    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    if !session::store::soft_delete(&state.db, id, current.user_id).await? {
        return Err(ApiError::not_found("会话不存在"));
    }

    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}

/// 提交一轮修改，返回任务号，客户端拿它查进度或订阅会话。
pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<ApiBody<session::TurnResult>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let turn = session::send_message(&state.db, current.user_id, id, req).await?;

    let background = state.clone();
    let job_id = turn.job_id;
    tokio::spawn(async move {
        let outcome = job::runner::execute(
            &background.db,
            &background.files,
            &background.http,
            &background.provider_key,
            job_id,
        )
        .await;

        if let Err(err) = outcome {
            tracing::error!(error = %err, %job_id, "会话任务执行异常");
        }
    });

    Ok(Json(ApiBody::ok(turn)))
}
