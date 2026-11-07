use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::error::{ApiBody, ApiError};
use crate::http::routes::AppState;

/// 用户端能选的模型。只暴露别称与代号，不泄露渠道信息。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicModel {
    pub code: String,
    pub display_name: String,
}

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<Vec<PublicModel>>>, ApiError> {
    // 同一个代号可能在多个渠道商下都有，用户端只出现一次：
    // 别称与顺序取排序最靠前的那条，管理员调顺序就能决定展示哪个名字。
    let rows = sqlx::query_as::<_, PublicModel>(
        "SELECT code, display_name FROM ( \
            SELECT DISTINCT ON (m.code) m.code, m.display_name, m.sort_order \
              FROM models m JOIN providers p ON p.id = m.provider_id \
             WHERE m.enabled AND p.enabled \
             ORDER BY m.code, m.sort_order, m.id \
         ) picked \
         ORDER BY picked.sort_order, picked.code",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiBody::ok(rows)))
}
