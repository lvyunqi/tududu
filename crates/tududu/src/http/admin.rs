use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use uuid::Uuid;

use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;
use crate::provider::{self, CreateChannelRequest, CreateProviderRequest, UpdateProviderRequest};

const ADMIN_TOKEN_HEADER: &str = "x-admin-token";

/// 管理接口的临时门禁：比对环境变量里配置的管理令牌。
///
/// 后续换成管理员账号体系后，这里替换成会话校验即可，调用方不用改。
pub fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(expected) = state.config.admin_token.as_deref() else {
        return Err(ApiError::forbidden(
            "管理接口未启用，请先配置 TUDUDU_ADMIN_TOKEN",
        ));
    };

    let provided = headers
        .get(ADMIN_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(ApiError::unauthorized());
    }

    Ok(())
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
