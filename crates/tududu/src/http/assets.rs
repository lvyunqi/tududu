use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth;
use crate::error::{ApiBody, ApiError};
use crate::http::auth::require_user;
use crate::http::routes::AppState;
use crate::storage::{self, store as asset_store};

/// 上传一张图片。表单字段名为 `file`。
pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<ApiBody<asset_store::AssetRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    let mut payload: Option<Vec<u8>> = None;
    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::warn!(error = %err, "解析上传表单失败");
        ApiError::bad_request("上传内容解析失败")
    })? {
        if field.name() != Some("file") {
            continue;
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|_| ApiError::bad_request("读取上传内容失败"))?;
        payload = Some(bytes.to_vec());
    }

    let bytes = payload.ok_or_else(|| ApiError::bad_request("表单里没有 file 字段"))?;

    let user = auth::store::find_user_by_id(&state.db, current.user_id)
        .await?
        .ok_or_else(ApiError::unauthorized)?;

    let asset = storage::save_upload(
        &state.db,
        &state.files,
        user.id,
        &user.short_code,
        &bytes,
    )
    .await?;

    Ok(Json(ApiBody::ok(asset)))
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
) -> Result<Json<ApiBody<Vec<asset_store::AssetRow>>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let rows = asset_store::list(&state.db, current.user_id, query.before, limit).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<asset_store::AssetRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    let asset = asset_store::find_owned(&state.db, id, current.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("图片不存在"))?;

    Ok(Json(ApiBody::ok(asset)))
}

/// 返回图片原始字节。只允许本人访问。
pub async fn file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let current = require_user(&state, &headers).await?;

    let asset = asset_store::find_owned(&state.db, id, current.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("图片不存在"))?;

    let bytes = state.files.read(&asset.object_key).await.map_err(|err| {
        tracing::error!(error = %err, key = %asset.object_key, "读取本地文件失败");
        ApiError::not_found("图片文件已丢失")
    })?;

    Ok(([(header::CONTENT_TYPE, asset.mime)], bytes).into_response())
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    if !asset_store::soft_delete(&state.db, id, current.user_id).await? {
        return Err(ApiError::not_found("图片不存在"));
    }

    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}
