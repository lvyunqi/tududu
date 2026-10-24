use std::net::SocketAddr;
use std::path::PathBuf;

/// 启动期配置。
///
/// 只承载进程启动必需的参数；业务配置（模型渠道、计费、支付等）存库，
/// 由管理后台维护，不放在这里。
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP 监听地址，来自 `TUDUDU_LISTEN`，默认 `0.0.0.0:8080`。
    pub listen: SocketAddr,
    /// 本地数据目录（上传的图片、缩略图等），来自 `TUDUDU_DATA_DIR`。
    pub data_dir: PathBuf,
    /// PostgreSQL 连接串，来自 `TUDUDU_DATABASE_URL`，必填。
    pub database_url: String,
    /// 连接池上限，来自 `TUDUDU_DB_MAX_CONNECTIONS`，默认 10。
    pub db_max_connections: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("缺少必需的环境变量 {name}")]
    Missing { name: &'static str },

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

        let database_url = required_env("TUDUDU_DATABASE_URL")?;

        let db_max_connections = match read_env("TUDUDU_DB_MAX_CONNECTIONS") {
            Some(raw) => raw.parse().map_err(|_| ConfigError::Env {
                name: "TUDUDU_DB_MAX_CONNECTIONS",
                value: raw,
            })?,
            None => 10,
        };

        Ok(Self {
            listen,
            data_dir,
            database_url,
            db_max_connections,
        })
    }
}

fn read_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn required_env(key: &'static str) -> Result<String, ConfigError> {
    read_env(key).ok_or(ConfigError::Missing { name: key })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_env_value_is_treated_as_absent() {
        assert_eq!(read_env("TUDUDU_DEFINITELY_UNSET_VAR"), None);
    }

    #[test]
    fn missing_required_env_is_reported() {
        let err = required_env("TUDUDU_DEFINITELY_UNSET_VAR").unwrap_err();
        assert!(err.to_string().contains("TUDUDU_DEFINITELY_UNSET_VAR"));
    }

    #[test]
    fn default_listen_is_all_interfaces() {
        assert_eq!(
            SocketAddr::from(([0, 0, 0, 0], 8080)).to_string(),
            "0.0.0.0:8080"
        );
    }
}
