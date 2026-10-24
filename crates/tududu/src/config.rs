use std::net::SocketAddr;
use std::path::PathBuf;

/// 启动期配置。
///
/// 只承载进程启动必需的参数；业务配置（模型渠道、计费、支付等）后续存库，
/// 由管理后台维护，不放在这里。
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP 监听地址，来自 `TUDUDU_LISTEN`，默认 `0.0.0.0:8080`。
    pub listen: SocketAddr,
    /// 本地数据目录（上传的图片、缩略图等），来自 `TUDUDU_DATA_DIR`。
    pub data_dir: PathBuf,
    /// 数据库连接串，来自 `TUDUDU_DATABASE_URL`。
    pub database_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("环境变量 {name} 的值不合法：{value}")]
    Env { name: &'static str, value: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let listen = match read_env("TUDUDU_LISTEN") {
            Some(raw) => raw.parse().map_err(|_| ConfigError::Env {
                name: "TUDUDU_LISTEN",
                value: raw,
            })?,
            None => SocketAddr::from(([0, 0, 0, 0], 8080)),
        };

        let data_dir = read_env("TUDUDU_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./data"));

        let database_url = read_env("TUDUDU_DATABASE_URL");

        Ok(Self {
            listen,
            data_dir,
            database_url,
        })
    }
}

fn read_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_env_value_is_treated_as_absent() {
        assert_eq!(read_env("TUDUDU_DEFINITELY_UNSET_VAR"), None);
    }

    #[test]
    fn default_listen_is_all_interfaces() {
        assert_eq!(
            SocketAddr::from(([0, 0, 0, 0], 8080)).to_string(),
            "0.0.0.0:8080"
        );
    }
}
