use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Serialize;
use sqlx::PgPool;

use crate::error::ApiBody;

/// 进程级共享状态。连接池句柄本身可 Clone。
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/health", get(health))
        .with_state(state)
}

/// 给负载均衡与容器探针用的最小健康检查，不碰数据库。
async fn healthz() -> &'static str {
    "ok"
}

#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
    database: &'static str,
}

async fn health(State(state): State<AppState>) -> Json<ApiBody<Health>> {
    let probe = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await;
    let database = if probe.is_ok() { "ok" } else { "down" };

    Json(ApiBody::ok(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        database,
    }))
}
