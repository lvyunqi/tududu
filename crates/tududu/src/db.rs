use std::time::Duration;

use anyhow::Context;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// 建立连接池。启动期只建一次，连不上就让进程直接退出。
pub async fn connect(url: &str, max_connections: u32) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await
        .context("连接数据库失败")
}

/// 执行内嵌的迁移脚本。
///
/// 迁移文件随二进制一起编译，部署时不需要额外带 SQL 文件，单文件即可上线。
pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("执行数据库迁移失败")
}
