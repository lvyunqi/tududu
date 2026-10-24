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
pub enum Command {
    /// 启动 HTTP API 服务
    Serve,
    /// 输出版本号
    Version,
}
