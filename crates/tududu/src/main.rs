use anyhow::Context;
use clap::Parser;
use sqlx::PgPool;

use tududu::cli::{Cli, Command};
use tududu::config::Config;
use tududu::{db, env_file, http, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 先读 .env，后面的 clap 与 Config 才能看到里面的变量
    env_file::load();

    let cli = Cli::parse();
    telemetry::init(&cli.log_filter);

    match cli.command {
        Command::Serve => {
            let config = Config::from_env().context("加载配置失败")?;
            let pool = open_database(&config).await?;
            let files = tududu::storage::local_store::LocalStore::new(config.data_dir.clone());

            // 上一个进程留下的任务不可能还在跑，启动时立刻回收，别让用户的钱挂着
            let recovered = tududu::tasks::recover_orphan_jobs(&pool, 0).await?;
            if recovered > 0 {
                tracing::warn!(count = recovered, "启动时回收了上次未完成的任务");
            }

            let stored = tududu::settings::load_stored(&pool).await?;
            let live = tududu::settings::resolve(&config, &stored);
            let settings = std::sync::Arc::new(std::sync::RwLock::new(live));

            tududu::tasks::spawn(pool.clone(), files, settings);
            http::serve(config, pool)
                .await
                .context("HTTP 服务异常退出")?;
        }
        Command::Migrate => {
            let config = Config::from_env().context("加载配置失败")?;
            let _pool = open_database(&config).await?;
            println!("数据库迁移完成");
        }
        Command::Version => {
            println!("tududu {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}

/// 建连接池并跑迁移。serve 和 migrate 两个子命令共用。
async fn open_database(config: &Config) -> anyhow::Result<PgPool> {
    let pool = db::connect(&config.database_url, config.db_max_connections).await?;
    db::migrate(&pool).await?;

    if let (Some(username), Some(password)) = (
        config.admin_username.as_deref(),
        config.admin_password.as_deref(),
    ) {
        tududu::admin_auth::bootstrap(&pool, username, password)
            .await
            .context("创建初始管理员失败")?;
    }

    Ok(pool)
}
