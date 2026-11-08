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
    limit_uploads(&state, current.user_id)?;

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
        state.live().user_quota_bytes,
    )
    .await?;

    Ok(Json(ApiBody::ok(asset)))
}

/// 单用户一小时最多传六十张。
fn limit_uploads(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    let key = format!("upload:{user_id}");

    state
        .limiter
        .check(&key, 60, std::time::Duration::from_secs(3600))
        .map_err(|wait| {
            ApiError::too_many_requests(format!(
                "上传太频繁，请 {} 分钟后再试",
                (wait.as_secs() / 60).max(1)
            ))
        })
}

#[derive(Debug, serde::Serialize)]
pub struct QuotaInfo {
    pub used_bytes: i64,
    pub limit_bytes: i64,
}

pub async fn quota(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<QuotaInfo>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let used_bytes = asset_store::used_bytes(&state.db, current.user_id).await?;
    let limit_bytes = state.live().user_quota_bytes;

    Ok(Json(ApiBody::ok(QuotaInfo {
        used_bytes,
        limit_bytes,
    })))
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

/// 返回图片原始字节。
///
/// 默认只允许本人访问；带 `e` 与 `t` 时改用签名校验，
/// 这样系统浏览器不带请求头也能取图。
#[derive(Debug, Deserialize)]
pub struct FileQuery {
    /// 传 thumb 时优先返回缩略图，没有缩略图就退回原图。
    pub size: Option<String>,
    /// 过期时间戳（秒）
    pub e: Option<i64>,
    /// 签名
    pub t: Option<String>,
}

pub async fn file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<FileQuery>,
) -> Result<Response, ApiError> {
    let signed = match (query.e, query.t.as_deref()) {
        (Some(expires), Some(sig)) => {
            expires > chrono::Utc::now().timestamp()
                && crate::sign::verify(&state.config.jwt_secret, &id.to_string(), expires, sig)
        }
        _ => false,
    };

    // 管理员要能看到被封禁的图，才方便核对；普通用户只能看自己的、且没被封的
    let is_admin = crate::http::admin::require_admin(&state, &headers).is_ok();

    let asset = if is_admin {
        asset_store::find_any(&state.db, id).await?
    } else if signed {
        asset_store::find_any(&state.db, id).await?
    } else {
        let current = require_user(&state, &headers).await?;
        asset_store::find_owned(&state.db, id, current.user_id).await?
    }
    .ok_or_else(|| ApiError::not_found("图片不存在"))?;

    // 封禁的图不外发：列表、灯箱、签名直链全部堵住
    if asset.status == 3 && !is_admin {
        return Err(ApiError::forbidden("这张图片已被封禁"));
    }

    // 三种尺寸：thumb 列表用、preview 点开看、其余（含下载）给原图
    let (key, mime, derived) = match query.size.as_deref() {
        Some("thumb") => match asset.thumb_key.as_deref() {
            Some(thumb) => (thumb.to_owned(), "image/jpeg", true),
            None => (asset.object_key.clone(), asset.mime.as_str(), false),
        },
        Some("preview") => match asset.preview_key.as_deref() {
            Some(preview) => (preview.to_owned(), "image/jpeg", true),
            // 老图没有预览图，第一次访问时补生成，之后就走缓存
            None => match build_preview(&state, &asset).await {
                Some(key) => (key, "image/jpeg", true),
                None => (asset.object_key.clone(), asset.mime.as_str(), false),
            },
        },
        _ => (asset.object_key.clone(), asset.mime.as_str(), false),
    };

    let bytes = match state.files.read(&key).await {
        Ok(bytes) => bytes,
        Err(err) if derived => {
            // 派生图丢了就退回原图，别让页面空着
            tracing::warn!(error = %err, key = %key, "读取派生图失败，回退原图");
            state.files.read(&asset.object_key).await.map_err(|err| {
                tracing::error!(error = %err, key = %asset.object_key, "读取本地文件失败");
                ApiError::not_found("图片文件已丢失")
            })?
        }
        Err(err) => {
            tracing::error!(error = %err, key = %key, "读取本地文件失败");
            return Err(ApiError::not_found("图片文件已丢失"));
        }
    };

    // 图片按 id 寻址，内容不会变，让浏览器放心缓存
    let cache = if derived {
        "private, max-age=86400"
    } else {
        "private, max-age=3600"
    };

    Ok((
        [(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache)],
        bytes,
    )
        .into_response())
}

/// 老资产没有预览图时按需生成，回写路径后复用。
async fn build_preview(state: &AppState, asset: &asset_store::AssetRow) -> Option<String> {
    let original = match state.files.read(&asset.object_key).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(error = %err, key = %asset.object_key, "读取原图失败，无法生成预览");
            return None;
        }
    };

    let preview = crate::storage::thumb::make_preview(&original)?;
    let key = format!("{}.preview.jpg", asset.object_key);

    if let Err(err) = state.files.save(&key, &preview).await {
        tracing::warn!(error = %err, key = %key, "写入预览图失败");
        return None;
    }

    if let Err(err) = asset_store::set_preview_key(&state.db, asset.id, &key).await {
        tracing::warn!(error = %err, id = %asset.id, "回写预览图路径失败");
    }

    Some(key)
}

/// 生成一个短时有效的下载直链，交给系统浏览器打开。
pub async fn download_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<DownloadLink>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    let asset = asset_store::find_owned(&state.db, id, current.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("图片不存在"))?;

    if asset.status == 3 {
        return Err(ApiError::forbidden("这张图片已被封禁"));
    }

    let expires_at = chrono::Utc::now().timestamp() + crate::sign::LINK_TTL_SECS;
    let signature = crate::sign::sign(&state.config.jwt_secret, &id.to_string(), expires_at);

    let url = format!("/api/v1/assets/{id}/file?e={expires_at}&t={signature}");

    Ok(Json(ApiBody::ok(DownloadLink { url, expires_at })))
}

#[derive(Debug, serde::Serialize)]
pub struct DownloadLink {
    /// 相对路径，前端自己拼上接口域名
    pub url: String,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<Uuid>,
}

/// 批量移入回收站。相册多选删除用。
pub async fn batch_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BatchDeleteRequest>,
) -> Result<Json<ApiBody<BatchDeleteResult>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    if req.ids.is_empty() {
        return Err(ApiError::bad_request("没有选中图片"));
    }

    if req.ids.len() > 200 {
        return Err(ApiError::bad_request("一次最多删 200 张"));
    }

    let removed = asset_store::soft_delete_many(&state.db, &req.ids, current.user_id).await?;

    Ok(Json(ApiBody::ok(BatchDeleteResult { removed })))
}

#[derive(Debug, serde::Serialize)]
pub struct BatchDeleteResult {
    pub removed: u64,
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
