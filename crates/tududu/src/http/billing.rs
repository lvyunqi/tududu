use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::billing::{self, AdjustCreditsRequest, UpdateConfigRequest, UpdatePricesRequest};
use crate::error::{ApiBody, ApiError};
use crate::http::admin::require_admin;
use crate::http::auth::require_user;
use crate::http::routes::AppState;

pub async fn wallet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<billing::store::WalletRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let found = billing::wallet(&state.db, current.user_id).await?;
    Ok(Json(ApiBody::ok(found)))
}

#[derive(Debug, Deserialize)]
pub struct LedgerQuery {
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

pub async fn ledger(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LedgerQuery>,
) -> Result<Json<ApiBody<Vec<billing::store::LedgerRow>>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let rows = billing::ledger(&state.db, current.user_id, query.before, limit).await?;
    Ok(Json(ApiBody::ok(rows)))
}

/// 单位名称是公开信息，前端要拿它显示余额。
pub async fn public_config(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<billing::store::ConfigRow>>, ApiError> {
    let found = billing::config(&state.db).await?;
    Ok(Json(ApiBody::ok(found)))
}

pub async fn list_prices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<Vec<billing::store::PriceRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = billing::store::list_prices(&state.db).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn update_prices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdatePricesRequest>,
) -> Result<Json<ApiBody<Vec<billing::store::PriceRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = billing::update_prices(&state.db, req).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn admin_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<billing::store::ConfigRow>>, ApiError> {
    require_admin(&state, &headers)?;
    let found = billing::config(&state.db).await?;
    Ok(Json(ApiBody::ok(found)))
}

pub async fn update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ApiBody<billing::store::ConfigRow>>, ApiError> {
    require_admin(&state, &headers)?;
    let found = billing::update_config(&state.db, req).await?;
    Ok(Json(ApiBody::ok(found)))
}

pub async fn adjust_credits(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<AdjustCreditsRequest>,
) -> Result<Json<ApiBody<billing::store::WalletRow>>, ApiError> {
    require_admin(&state, &headers)?;

    if req.amount == 0 {
        return Err(ApiError::bad_request("调整数量不能为 0"));
    }

    let remark = req.remark.as_deref().unwrap_or("管理员调整");
    if req.amount > 0 {
        billing::grant(&state.db, user_id, req.amount, remark).await?;
    } else {
        billing::charge(&state.db, user_id, -req.amount, None, None, remark).await?;
    }

    let found = billing::wallet(&state.db, user_id).await?;
    Ok(Json(ApiBody::ok(found)))
}
