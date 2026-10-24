use axum::Json;
use axum::extract::State;

use crate::admin_auth::{self, LoginRequest, LoginResponse};
use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiBody<LoginResponse>>, ApiError> {
    let result = admin_auth::login(&state.db, &state.config.jwt_secret, req).await?;
    Ok(Json(ApiBody::ok(result)))
}
