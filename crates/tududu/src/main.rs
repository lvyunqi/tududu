use anyhow::Context;
use clap::Parser;

use tududu::cli::{Cli, Command};
use tududu::config::Config;
use tududu::{http, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    telemetry::init(&cli.log_filter);

    match cli.command {
        Command::Serve => {
            let config = Config::from_env().context("加载配置失败")?;
            http::serve(config).await.context("HTTP 服务异常退出")?;
        }
        Command::Version => {
            println!("tududu {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
