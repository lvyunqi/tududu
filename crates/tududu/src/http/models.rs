use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;

/// 用户端能选的模型。只暴露代码与显示名，不泄露渠道信息。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicModel {
    pub code: String,
    pub display_name: String,
}

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<Vec<PublicModel>>>, ApiError> {
    let rows = sqlx::query_as::<_, PublicModel>(
        "SELECT DISTINCT ON (m.code) m.code, m.display_name \
           FROM models m JOIN providers p ON p.id = m.provider_id \
          WHERE m.enabled AND p.enabled \
          ORDER BY m.code, m.display_name",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiBody::ok(rows)))
}
