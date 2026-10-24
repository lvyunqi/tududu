use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::admin::{self, store as admin_store};
use crate::error::{ApiBody, ApiError};
use crate::http::admin::require_admin;
use crate::http::routes::AppState;
use crate::plaza::store as plaza_store;
use crate::tasks;

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

fn page_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(20).clamp(1, 100)
}

/// 手动跑一轮对账，返回关闭的过期订单数与账本异常。
pub async fn reconcile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<tasks::ReconcileReport>>, ApiError> {
    require_admin(&state, &headers)?;
    let report = tasks::run_once(&state.db).await?;
    Ok(Json(ApiBody::ok(report)))
}

pub async fn overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<admin_store::OverviewRow>>, ApiError> {
    require_admin(&state, &headers)?;
    let row = admin_store::overview(&state.db).await?;
    Ok(Json(ApiBody::ok(row)))
}

pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApiBody<Vec<admin_store::AdminUserRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = admin_store::list_users(&state.db, query.before, page_limit(query.limit)).await?;
    Ok(Json(ApiBody::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct UserStatusRequest {
    pub status: i16,
}

pub async fn set_user_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UserStatusRequest>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let operator = require_admin(&state, &headers)?;

    if !admin::is_valid_user_status(req.status) {
        return Err(ApiError::bad_request("状态只能是 1 正常、2 禁言、3 封禁"));
    }

    if !admin_store::set_user_status(&state.db, id, req.status).await? {
        return Err(ApiError::not_found("用户不存在"));
    }

    let detail = format!("status={}", req.status);
    let _ = admin_store::record_audit(
        &state.db,
        operator.id,
        "user_status",
        "user",
        &id.to_string(),
        &detail,
    )
    .await;

    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct OrderQuery {
    pub status: Option<i16>,
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn list_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OrderQuery>,
) -> Result<Json<ApiBody<Vec<admin_store::AdminOrderRow>>>, ApiError> {
    require_admin(&state, &headers)?;

    let rows = admin_store::list_orders(
        &state.db,
        query.status,
        query.before,
        page_limit(query.limit),
    )
    .await?;

    Ok(Json(ApiBody::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct JobQuery {
    pub status: Option<i16>,
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn list_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<JobQuery>,
) -> Result<Json<ApiBody<Vec<admin_store::AdminJobRow>>>, ApiError> {
    require_admin(&state, &headers)?;

    let rows = admin_store::list_jobs(
        &state.db,
        query.status,
        query.before,
        page_limit(query.limit),
    )
    .await?;

    Ok(Json(ApiBody::ok(rows)))
}

#[derive(Debug, Serialize)]
pub struct JobDetail {
    pub job: admin_store::AdminJobRow,
    pub attempts: Vec<admin_store::AttemptRow>,
}

pub async fn job_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<JobDetail>>, ApiError> {
    require_admin(&state, &headers)?;

    let job = admin_store::find_job(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("任务不存在"))?;

    let attempts = admin_store::list_attempts(&state.db, id).await?;
    Ok(Json(ApiBody::ok(JobDetail { job, attempts })))
}

#[derive(Debug, Deserialize)]
pub struct PromptQuery {
    pub status: Option<i16>,
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn list_prompts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PromptQuery>,
) -> Result<Json<ApiBody<Vec<plaza_store::PromptRow>>>, ApiError> {
    require_admin(&state, &headers)?;

    let status = query.status.unwrap_or(plaza_store::STATUS_PUBLISHED);
    let rows =
        plaza_store::list_by_status(&state.db, status, query.before, page_limit(query.limit))
            .await?;

    Ok(Json(ApiBody::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct PromptStatusRequest {
    pub status: i16,
}

pub async fn set_prompt_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
    Json(req): Json<PromptStatusRequest>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let operator = require_admin(&state, &headers)?;

    if req.status != plaza_store::STATUS_PUBLISHED && req.status != plaza_store::STATUS_HIDDEN {
        return Err(ApiError::bad_request("状态只能是 2 已发布或 3 已下架"));
    }

    if !plaza_store::set_status(&state.db, &code, req.status).await? {
        return Err(ApiError::not_found("作品不存在"));
    }

    let detail = format!("status={}", req.status);
    let _ = admin_store::record_audit(
        &state.db,
        operator.id,
        "prompt_status",
        "prompt",
        &code,
        &detail,
    )
    .await;

    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}
