pub mod admin;
pub mod assets;
pub mod auth;
pub mod billing;
pub mod jobs;
pub mod payment;
pub mod plaza;
pub mod routes;
pub mod sessions;

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

    let state = routes::AppState {
        db,
        config: Arc::new(config),
        provider_key,
        http: reqwest::Client::new(),
        files,
    };

    axum::serve(listener, routes::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum serve 失败")?;

    Ok(())
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
