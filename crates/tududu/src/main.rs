use anyhow::Context;
use clap::Parser;
use sqlx::PgPool;

use tududu::cli::{Cli, Command};
use tududu::config::Config;
use tududu::{db, http, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    telemetry::init(&cli.log_filter);

    match cli.command {
        Command::Serve => {
            let config = Config::from_env().context("加载配置失败")?;
            let pool = open_database(&config).await?;
            http::serve(config, pool).await.context("HTTP 服务异常退出")?;
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
    Ok(pool)
}
