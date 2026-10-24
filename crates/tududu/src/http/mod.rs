pub mod routes;

use anyhow::Context;
use sqlx::PgPool;

use crate::config::Config;

pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("监听 {} 失败", config.listen))?;

    tracing::info!(
        addr = %config.listen,
        data_dir = %config.data_dir.display(),
        "http server listening"
    );

    let state = routes::AppState { db };

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
