use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::{ApiBody, ApiError};
use crate::http::auth::require_user;
use crate::http::routes::AppState;
use crate::plaza::{self, PublishRequest};

pub async fn categories(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<Vec<plaza::store::CategoryRow>>>, ApiError> {
    let rows = plaza::store::list_categories(&state.db).await?;
    Ok(Json(ApiBody::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub category_id: Option<i64>,
    /// 搜索词，同时匹配标题与提示词
    pub q: Option<String>,
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ApiBody<Vec<plaza::store::PromptRow>>>, ApiError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let keyword = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));

    let rows = plaza::store::list_published(
        &state.db,
        query.category_id,
        keyword.as_deref(),
        query.before,
        limit,
    )
    .await?;

    Ok(Json(ApiBody::ok(rows)))
}

pub async fn publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PublishRequest>,
) -> Result<Json<ApiBody<plaza::store::PromptRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let row = plaza::publish(&state.db, current.user_id, req).await?;
    Ok(Json(ApiBody::ok(row)))
}

pub async fn detail(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiBody<plaza::store::PromptRow>>, ApiError> {
    let found = plaza::detail(&state.db, &code).await?;
    Ok(Json(ApiBody::ok(found)))
}

pub async fn like(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<ApiBody<plaza::LikeResult>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let result = plaza::toggle_like(&state.db, current.user_id, &code).await?;
    Ok(Json(ApiBody::ok(result)))
}

pub async fn unpublish(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    plaza::unpublish(&state.db, current.user_id, &code).await?;

    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct MineQuery {
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn mine(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MineQuery>,
) -> Result<Json<ApiBody<Vec<plaza::store::PromptRow>>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let rows =
        plaza::store::list_by_author(&state.db, current.user_id, query.before, limit).await?;
    Ok(Json(ApiBody::ok(rows)))
}
