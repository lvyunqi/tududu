//! 版本检测与安装包下载中转。
//!
//! 客户端只跟本站打交道，不去碰 GitHub：一来国内直连慢，二来把仓库地址藏起来。

use axum::Json;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;
use crate::update::UpdateService;

#[derive(Debug, Deserialize)]
pub struct VersionQuery {
    /// android 或 linux，默认 android
    pub platform: Option<String>,
    /// 客户端当前版本，比如 0.1.13
    pub current: Option<String>,
}

/// 查最新版本。公开接口，App 启动时调一次。
pub async fn version(
    State(state): State<AppState>,
    Query(query): Query<VersionQuery>,
) -> Result<Json<ApiBody<crate::update::VersionInfo>>, ApiError> {
    let platform = query.platform.as_deref().unwrap_or("android");
    let current = query.current.as_deref().unwrap_or_default();

    let info = state.updates.version(platform, current).await.map_err(|err| {
        tracing::warn!(error = %err, "查询最新版本失败");
        ApiError::new(StatusCode::BAD_GATEWAY, 40003, "暂时查不到版本信息")
    })?;

    Ok(Json(ApiBody::ok(info)))
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub platform: Option<String>,
}

/// 下载中转：把 GitHub 的安装包流式转发给客户端，带 Content-Length 方便做进度条。
pub async fn download(
    State(state): State<AppState>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
    let platform = query.platform.as_deref().unwrap_or("android");

    let release = state.updates.latest().await.map_err(|err| {
        tracing::warn!(error = %err, "下载时查询 release 失败");
        ApiError::new(StatusCode::BAD_GATEWAY, 40003, "暂时拿不到安装包")
    })?;

    let asset = UpdateService::asset_for(&release, platform)
        .ok_or_else(|| ApiError::not_found("这个版本没有对应的安装包"))?;

    let upstream = state
        .http
        .get(&asset.browser_download_url)
        .header("User-Agent", "tududu-update")
        .send()
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, url = %asset.browser_download_url, "拉取安装包失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40003, "拉取安装包失败")
        })?;

    if !upstream.status().is_success() {
        tracing::warn!(status = %upstream.status(), "上游安装包地址返回异常");
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            40003,
            "拉取安装包失败",
        ));
    }

    let total = upstream.content_length().unwrap_or(asset.size.max(0) as u64);
    let body = Body::from_stream(upstream.bytes_stream());

    let disposition = format!("attachment; filename=\"{}\"", asset.name);

    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::CONTENT_LENGTH, total.to_string()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, "public, max-age=300".to_owned()),
        ],
        body,
    )
        .into_response())
}
