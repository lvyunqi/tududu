use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use uuid::Uuid;

use crate::auth::token;
use crate::auth::{self, AuthResponse, LoginRequest, RefreshRequest, RegisterRequest, RequestMeta};
use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;

/// 当前请求的登录身份。
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiBody<AuthResponse>>, ApiError> {
    let meta = meta_from_headers(&headers);
    let resp = auth::register(&state.db, &state.config, req, meta).await?;
    Ok(Json(ApiBody::ok(resp)))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiBody<AuthResponse>>, ApiError> {
    let meta = meta_from_headers(&headers);
    let resp = auth::login(&state.db, &state.config, req, meta).await?;
    Ok(Json(ApiBody::ok(resp)))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<ApiBody<AuthResponse>>, ApiError> {
    let resp = auth::refresh(&state.db, &state.config, req).await?;
    Ok(Json(ApiBody::ok(resp)))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<()>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    auth::logout(&state.db, current.session_id).await?;
    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}

pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<auth::UserInfo>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let profile = auth::profile(&state.db, current.user_id).await?;
    Ok(Json(ApiBody::ok(profile)))
}

/// 从 Authorization 头解析并校验访问令牌。
pub async fn require_user(state: &AppState, headers: &HeaderMap) -> Result<CurrentUser, ApiError> {
    let raw = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(ApiError::unauthorized)?;

    let token = raw
        .strip_prefix("Bearer ")
        .ok_or_else(ApiError::unauthorized)?
        .trim();

    let claims = token::verify_access(token, &state.config.jwt_secret)
        .map_err(|_| ApiError::unauthorized())?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::unauthorized())?;
    let session_id = Uuid::parse_str(&claims.sid).map_err(|_| ApiError::unauthorized())?;

    Ok(CurrentUser {
        user_id,
        session_id,
    })
}

fn meta_from_headers(headers: &HeaderMap) -> RequestMeta {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };

    RequestMeta {
        device_id: get("x-device-id").unwrap_or_else(|| "unknown".to_owned()),
        device_name: get("x-device-name"),
        platform: get("x-platform"),
        app_version: get("x-app-version"),
        ip: get("x-forwarded-for").or_else(|| get("x-real-ip")),
        user_agent: get("user-agent"),
    }
}
