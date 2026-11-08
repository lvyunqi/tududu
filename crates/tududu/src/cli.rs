use clap::{Parser, Subcommand};

/// 图嘟嘟服务端
#[derive(Debug, Parser)]
#[command(name = "tududu", version, about = "图嘟嘟服务端", long_about = None)]
pub struct Cli {
    /// 日志过滤规则，语法与 RUST_LOG 相同
    #[arg(long, global = true, env = "TUDUDU_LOG", default_value = "info")]
    pub log_filter: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum UpdateAction {
    /// 看看有没有新版本
    Check {
        /// 服务端地址，默认取 TUDUDU_BASE_URL 或本机 8080
        #[arg(long)]
        url: Option<String>,
    },
    /// 下载并替换当前程序
    Install {
        #[arg(long)]
        url: Option<String>,
        /// 不再二次确认
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 启动 HTTP API 服务，启动时自动执行数据库迁移
    Serve,
    /// 只执行数据库迁移然后退出
    Migrate,
    /// 查新版本或就地更新（配合启动器使用）
    Update {
        #[command(subcommand)]
        action: UpdateAction,
    },
    /// 输出版本号
    Version,
}
