pub mod admin;
pub mod admin_auth;
pub mod admin_ops;
pub mod assets;
pub mod auth;
pub mod billing;
pub mod jobs;
pub mod models;
pub mod payment;
pub mod plaza;
pub mod qrcode;
pub mod routes;
pub mod sessions;
pub mod static_files;

use std::sync::Arc;

use anyhow::Context;
use sqlx::PgPool;

use crate::config::Config;

pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let listen = config.listen;
    let data_dir = config.data_dir.clone();

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("监听 {listen} 失败"))?;

    tracing::info!(
        addr = %listen,
        data_dir = %data_dir.display(),
        "http server listening"
    );

    let provider_key = crate::provider::crypto::derive_key(&config.jwt_secret);
    let files = crate::storage::local_store::LocalStore::new(data_dir.clone());
    let cors_origins = config.cors_origins.clone();

    // 数据库里的值覆盖环境变量，然后整份放进状态里
    let stored = crate::settings::load_stored(&db).await?;
    let live = crate::settings::resolve(&config, &stored);

    tracing::info!(
        access_ttl = live.access_ttl_secs,
        quota = live.user_quota_bytes,
        trash_days = live.trash_retention_days,
        "运行时配置已加载"
    );

    let state = routes::AppState {
        db,
        config: Arc::new(config),
        provider_key,
        http: reqwest::Client::new(),
        files,
        limiter: Arc::new(crate::ratelimit::RateLimiter::new()),
        settings: Arc::new(std::sync::RwLock::new(live)),
    };

    let app = routes::router(state).layer(cors_layer(&cors_origins));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum serve 失败")?;

    Ok(())
}

/// 跨域策略。列表为空时不加这一层，等价于只允许同源。
fn cors_layer(origins: &[String]) -> tower_http::cors::CorsLayer {
    use axum::http::{HeaderValue, Method, header};
    use tower_http::cors::CorsLayer;

    let allowed: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    if allowed.is_empty() {
        return CorsLayer::new();
    }

    CorsLayer::new()
        .allow_origin(allowed)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(600))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
