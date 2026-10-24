use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::{get, patch, post};
use serde::Serialize;
use sqlx::PgPool;

use crate::config::Config;
use crate::error::ApiBody;
use crate::http::{admin, auth};

/// 进程级共享状态。连接池句柄可 Clone，配置只读所以放进 Arc。
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    /// 渠道密钥加解密用的密钥，启动时由主密钥派生。
    pub provider_key: [u8; 32],
    /// 访问上游用的 HTTP 客户端，全局复用一个连接池。
    pub http: reqwest::Client,
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
        .route(
            "/api/v1/admin/providers",
            get(admin::list_providers).post(admin::create_provider),
        )
        .route(
            "/api/v1/admin/providers/{id}",
            patch(admin::update_provider).delete(admin::delete_provider),
        )
        .route(
            "/api/v1/admin/providers/{id}/channels",
            get(admin::list_channels).post(admin::create_channel),
        )
        .route(
            "/api/v1/admin/providers/{id}/models",
            get(admin::list_models),
        )
        .route(
            "/api/v1/admin/providers/{id}/models/refresh",
            post(admin::refresh_models),
        )
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
