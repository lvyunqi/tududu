//! 读取 `.env`，让本地与小型部署不必手工 export 一堆变量。
//!
//! 顺序很重要：先读文件，再解析命令行和配置。真实环境变量优先级高于文件，
//! 所以容器里显式传入的值不会被文件覆盖。

use std::path::Path;

/// 默认在当前目录找 `.env`；`TUDUDU_ENV_FILE` 可以指定别的路径。
pub fn load() {
    let explicit = std::env::var("TUDUDU_ENV_FILE")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    match explicit {
        Some(path) => match dotenvy::from_path(&path) {
            Ok(_) => tracing::info!(%path, "已加载环境变量文件"),
            Err(err) => tracing::warn!(%path, error = %err, "环境变量文件加载失败"),
        },
        None => {
            if Path::new(".env").exists() {
                match dotenvy::dotenv() {
                    Ok(_) => tracing::info!("已加载 .env"),
                    Err(err) => tracing::warn!(error = %err, ".env 加载失败"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_not_fatal() {
        // 没配 TUDUDU_ENV_FILE 且当前目录没有 .env 时，加载应该安静地跳过
        load();
    }
}
