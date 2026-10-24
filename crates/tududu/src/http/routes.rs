use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::{get, post};
use serde::Serialize;
use sqlx::PgPool;

use crate::config::Config;
use crate::error::ApiBody;
use crate::http::auth;

/// 进程级共享状态。连接池句柄可 Clone，配置只读所以放进 Arc。
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/me", get(auth::me))
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
