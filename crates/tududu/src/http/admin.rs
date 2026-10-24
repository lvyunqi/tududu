use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use uuid::Uuid;

use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;
use crate::provider::{self, CreateChannelRequest, CreateProviderRequest, UpdateProviderRequest};

const ADMIN_TOKEN_HEADER: &str = "x-admin-token";

/// 管理接口的调用方。走账号登录时有 id，走旧令牌时只有名字。
#[derive(Debug, Clone)]
pub struct AdminIdentity {
    pub id: Option<Uuid>,
    pub name: String,
}

/// 先看 Authorization 里有没有管理员令牌，没有再退回旧的 x-admin-token。
///
/// 旧令牌保留是为了过渡期：已经配好的部署不用立刻改，新部署直接用账号登录。
pub fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<AdminIdentity, ApiError> {
    if let Some(identity) = from_bearer(state, headers) {
        return Ok(identity);
    }

    let Some(expected) = state.config.admin_token.as_deref() else {
        return Err(ApiError::forbidden(
            "管理接口未启用，请用管理员账号登录，或配置 TUDUDU_ADMIN_TOKEN",
        ));
    };

    let provided = headers
        .get(ADMIN_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(ApiError::unauthorized());
    }

    Ok(AdminIdentity {
        id: None,
        name: "旧令牌".to_owned(),
    })
}

fn from_bearer(state: &AppState, headers: &HeaderMap) -> Option<AdminIdentity> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;

    let token = raw.strip_prefix("Bearer ")?;
    let claims = crate::admin_auth::verify(&state.config.jwt_secret, token.trim())?;

    Some(AdminIdentity {
        id: Uuid::parse_str(&claims.sub).ok(),
        name: claims.name,
    })
}

pub async fn list_providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<Vec<provider::store::ProviderRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = provider::list_providers(&state.db).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn create_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<ApiBody<provider::store::ProviderRow>>, ApiError> {
    require_admin(&state, &headers)?;
    let row = provider::create_provider(&state.db, req).await?;
    Ok(Json(ApiBody::ok(row)))
}

pub async fn update_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<ApiBody<provider::store::ProviderRow>>, ApiError> {
    require_admin(&state, &headers)?;
    let row = provider::update_provider(&state.db, id, req).await?;
    Ok(Json(ApiBody::ok(row)))
}

pub async fn delete_provider(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<()>>, ApiError> {
    require_admin(&state, &headers)?;
    provider::delete_provider(&state.db, id).await?;
    Ok(Json(ApiBody {
        code: 0,
        message: "ok".to_owned(),
        data: None,
    }))
}

pub async fn create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ApiBody<provider::store::ChannelRow>>, ApiError> {
    require_admin(&state, &headers)?;
    let row = provider::create_channel(&state.db, &state.provider_key, id, req).await?;
    Ok(Json(ApiBody::ok(row)))
}

pub async fn list_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<Vec<provider::store::ChannelRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = provider::list_channels(&state.db, id).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn refresh_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<Vec<provider::store::ModelRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = provider::refresh_models(&state.db, &state.http, &state.provider_key, id).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn list_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<Vec<provider::store::ModelRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = provider::list_models(&state.db, id).await?;
    Ok(Json(ApiBody::ok(rows)))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b) {
        diff |= left ^ right;
    }

    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_behaves_like_equality() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
