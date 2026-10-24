use axum::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;

use crate::error::ApiBody;

pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/health", get(health))
}

/// 给负载均衡与容器探针用的最小健康检查。
async fn healthz() -> &'static str {
    "ok"
}

#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<ApiBody<Health>> {
    Json(ApiBody::ok(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}
