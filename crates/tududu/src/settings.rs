//! 后台可以在线修改的运行时配置。
//!
//! 只放「改了能立刻生效」的东西：令牌有效期、配额、回收站天数、对外地址。
//! 数据库连接串、签名密钥、监听地址这类启动期参数改不了，留在环境变量里。
//!
//! 优先级：数据库里的值 > 环境变量 > 内置默认值。这样已经配了环境变量的部署
//! 也能在后台覆盖，不用去改机器上的文件。

use std::collections::HashMap;

use serde::Serialize;
use sqlx::PgPool;

use crate::config::Config;
use crate::error::ApiError;

/// 当前生效的值。整体克隆很便宜，读的时候直接拷一份走。
#[derive(Debug, Clone)]
pub struct LiveSettings {
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
    pub user_quota_bytes: i64,
    pub trash_retention_days: i32,
    pub public_base_url: Option<String>,
}

pub struct SettingSpec {
    pub key: &'static str,
    pub description: &'static str,
    pub kind: SettingKind,
    pub min: i64,
    pub max: i64,
}

#[derive(PartialEq, Eq)]
pub enum SettingKind {
    Int,
    Url,
}

pub const SPECS: &[SettingSpec] = &[
    SettingSpec {
        key: "access_ttl_secs",
        description: "访问令牌有效期（秒）",
        kind: SettingKind::Int,
        min: 60,
        max: 86_400,
    },
    SettingSpec {
        key: "refresh_ttl_secs",
        description: "刷新令牌有效期（秒）",
        kind: SettingKind::Int,
        min: 3_600,
        max: 31_536_000,
    },
    SettingSpec {
        key: "user_quota_bytes",
        description: "单用户存储上限（字节）",
        kind: SettingKind::Int,
        min: 0,
        max: 1_099_511_627_776,
    },
    SettingSpec {
        key: "trash_retention_days",
        description: "回收站保留天数",
        kind: SettingKind::Int,
        min: 1,
        max: 3_650,
    },
    SettingSpec {
        key: "public_base_url",
        description: "对外访问地址，用于拼接支付回调；留空表示未配置",
        kind: SettingKind::Url,
        min: 0,
        max: 0,
    },
];

#[derive(Debug, Serialize)]
pub struct SettingView {
    pub key: String,
    pub value: String,
    pub description: String,
    pub from_db: bool,
}

impl LiveSettings {
    /// 环境变量或内置默认值，作为数据库值的兜底。
    pub fn from_config(config: &Config) -> Self {
        Self {
            access_ttl_secs: config.access_token_ttl_secs,
            refresh_ttl_secs: config.refresh_token_ttl_secs,
            user_quota_bytes: config.user_quota_bytes,
            trash_retention_days: config.trash_retention_days,
            public_base_url: config.public_base_url.clone(),
        }
    }

    /// 按 key 改一项，返回新值。校验不通过就报错，原来的值不动。
    pub fn with(&self, key: &str, raw: &str) -> Result<Self, ApiError> {
        let spec = SPECS
            .iter()
            .find(|item| item.key == key)
            .ok_or_else(|| ApiError::bad_request(format!("未知配置项：{key}")))?;

        let raw = raw.trim();
        let mut next = self.clone();

        match spec.kind {
            SettingKind::Int => {
                let parsed: i64 = raw
                    .parse()
                    .map_err(|_| ApiError::bad_request(format!("{} 需要整数", spec.key)))?;

                if parsed < spec.min || parsed > spec.max {
                    return Err(ApiError::bad_request(format!(
                        "{} 需要在 {} 到 {} 之间",
                        spec.key, spec.min, spec.max
                    )));
                }

                match spec.key {
                    "access_ttl_secs" => next.access_ttl_secs = parsed,
                    "refresh_ttl_secs" => next.refresh_ttl_secs = parsed,
                    "user_quota_bytes" => next.user_quota_bytes = parsed,
                    "trash_retention_days" => next.trash_retention_days = parsed as i32,
                    _ => return Err(ApiError::bad_request("配置项没有对应的字段")),
                }
            }
            SettingKind::Url => {
                if raw.is_empty() {
                    next.public_base_url = None;
                } else {
                    if !(raw.starts_with("http://") || raw.starts_with("https://")) {
                        return Err(ApiError::bad_request("地址需要以 http:// 或 https:// 开头"));
                    }
                    next.public_base_url = Some(raw.trim_end_matches('/').to_owned());
                }
            }
        }

        Ok(next)
    }

    /// 把一项读成字符串，给后台展示用。
    pub fn value_of(&self, key: &str) -> String {
        match key {
            "access_ttl_secs" => self.access_ttl_secs.to_string(),
            "refresh_ttl_secs" => self.refresh_ttl_secs.to_string(),
            "user_quota_bytes" => self.user_quota_bytes.to_string(),
            "trash_retention_days" => self.trash_retention_days.to_string(),
            "public_base_url" => self.public_base_url.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }
}

pub async fn load_stored(db: &PgPool) -> sqlx::Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(db)
        .await?;

    Ok(rows.into_iter().collect())
}

/// 数据库值覆盖环境变量。库里存的值如果解析失败就忽略，避免一条脏数据让服务起不来。
pub fn resolve(config: &Config, stored: &HashMap<String, String>) -> LiveSettings {
    let mut live = LiveSettings::from_config(config);

    for (key, raw) in stored {
        if let Ok(next) = live.with(key, raw) {
            live = next;
        } else {
            tracing::warn!(%key, %raw, "数据库里的配置值不合法，已忽略");
        }
    }

    live
}

pub async fn describe(
    db: &PgPool,
    config: &Config,
    live: &LiveSettings,
) -> sqlx::Result<Vec<SettingView>> {
    let stored = load_stored(db).await?;
    let fallback = LiveSettings::from_config(config);

    Ok(SPECS
        .iter()
        .map(|spec| {
            let from_db = stored.contains_key(spec.key);
            let value = if from_db {
                live.value_of(spec.key)
            } else {
                fallback.value_of(spec.key)
            };

            SettingView {
                key: spec.key.to_owned(),
                value,
                description: spec.description.to_owned(),
                from_db,
            }
        })
        .collect())
}

pub async fn save(
    db: &PgPool,
    key: &str,
    raw: &str,
    current: &LiveSettings,
) -> Result<LiveSettings, ApiError> {
    let next = current.with(key, raw)?;

    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ($1, $2) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(key)
    .bind(raw.trim())
    .execute(db)
    .await?;

    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LiveSettings {
        LiveSettings {
            access_ttl_secs: 900,
            refresh_ttl_secs: 2_592_000,
            user_quota_bytes: 2_147_483_648,
            trash_retention_days: 30,
            public_base_url: None,
        }
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(sample().with("access_ttl_secs", "10").is_err());
        assert!(sample().with("access_ttl_secs", "999999").is_err());
        assert!(sample().with("access_ttl_secs", "abc").is_err());
    }

    #[test]
    fn accepts_valid_value() {
        let next = sample().with("access_ttl_secs", "1800").expect("应当通过");
        assert_eq!(next.access_ttl_secs, 1800);
    }

    #[test]
    fn url_must_have_scheme() {
        assert!(sample().with("public_base_url", "api.example.com").is_err());

        let next = sample()
            .with("public_base_url", "https://api.example.com/")
            .expect("应当通过");

        assert_eq!(
            next.public_base_url.as_deref(),
            Some("https://api.example.com")
        );
    }

    #[test]
    fn empty_url_clears_it() {
        let mut base = sample();
        base.public_base_url = Some("https://a.example.com".to_owned());

        let next = base.with("public_base_url", "").expect("应当通过");
        assert!(next.public_base_url.is_none());
    }

    #[test]
    fn unknown_key_is_rejected() {
        assert!(sample().with("nope", "1").is_err());
    }
}
