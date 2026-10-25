//! 账号体系：注册、登录、刷新令牌轮换与登出。

pub mod password;
pub mod store;
pub mod token;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::error::ApiError;
use crate::settings::LiveSettings;

/// 建会话时采集的设备与环境信息，用于设备列表和风控。
#[derive(Debug, Clone, Default)]
pub struct RequestMeta {
    pub device_id: String,
    pub device_name: Option<String>,
    pub platform: Option<String>,
    pub app_version: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub password: String,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub short_code: String,
    pub nickname: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<store::UserRow> for UserInfo {
    fn from(row: store::UserRow) -> Self {
        Self {
            id: row.id.to_string(),
            short_code: row.short_code,
            nickname: row.nickname,
            phone: row.phone,
            email: row.email,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

pub async fn register(
    db: &PgPool,
    config: &Config,
    live: &LiveSettings,
    req: RegisterRequest,
    meta: RequestMeta,
) -> Result<AuthResponse, ApiError> {
    validate_password(&req.password)?;

    let phone = req.phone.as_deref().and_then(normalize_phone);
    let email = req.email.as_deref().and_then(normalize_email);

    if phone.is_none() && email.is_none() {
        return Err(ApiError::bad_request(
            "手机号或邮箱至少填一个，且格式要正确",
        ));
    }

    ensure_phone_available(db, phone.as_deref()).await?;
    ensure_email_available(db, email.as_deref()).await?;

    let password_hash = password::hash(&req.password).map_err(|err| {
        tracing::error!(error = %err, "生成密码散列失败");
        ApiError::internal("注册失败，请稍后再试")
    })?;

    let provided_nickname = req
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(24).collect::<String>());

    let mut collisions = 0;
    let user = loop {
        let short_code = new_short_code();
        let nickname = provided_nickname
            .clone()
            .unwrap_or_else(|| format!("图嘟嘟用户{}", &short_code[..4]));

        let new_user = store::NewUser {
            short_code: &short_code,
            nickname: &nickname,
            phone: phone.as_deref(),
            email: email.as_deref(),
            password_hash: &password_hash,
            register_ip: meta.ip.as_deref(),
        };

        match store::insert_user(db, &new_user).await {
            Ok(user) => break user,
            Err(err) if store::is_unique_violation(&err) => {
                collisions += 1;
                if collisions > 2 {
                    return Err(ApiError::internal("创建账号失败，请稍后再试"));
                }
                ensure_phone_available(db, phone.as_deref()).await?;
                ensure_email_available(db, email.as_deref()).await?;
            }
            Err(err) => return Err(err.into()),
        }
    };

    crate::billing::store::ensure_wallet(db, user.id).await?;

    let (access_token, refresh_token) = issue_tokens(db, config, live, user.id, &meta).await?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        expires_in: live.access_ttl_secs,
        user: user.into(),
    })
}

pub async fn login(
    db: &PgPool,
    config: &Config,
    live: &LiveSettings,
    req: LoginRequest,
    meta: RequestMeta,
) -> Result<AuthResponse, ApiError> {
    let account = req.account.trim();

    let found = if let Some(phone) = normalize_phone(account) {
        store::find_user_by_phone(db, &phone).await?
    } else if let Some(email) = normalize_email(account) {
        store::find_user_by_email(db, &email).await?
    } else {
        return Err(ApiError::bad_request("账号格式不正确"));
    };

    let user =
        found.ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, 20002, "账号或密码错误"))?;

    let stored = user
        .password_hash
        .as_deref()
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, 20002, "账号或密码错误"))?;

    if !password::verify(&req.password, stored) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            20002,
            "账号或密码错误",
        ));
    }

    ensure_active(user.status)?;

    let (access_token, refresh_token) = issue_tokens(db, config, live, user.id, &meta).await?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        expires_in: live.access_ttl_secs,
        user: user.into(),
    })
}

/// 刷新时同时轮换刷新令牌，旧令牌立即失效。
pub async fn refresh(
    db: &PgPool,
    config: &Config,
    live: &LiveSettings,
    req: RefreshRequest,
) -> Result<AuthResponse, ApiError> {
    let current_hash = token::hash_refresh_token(&req.refresh_token);
    let session = store::find_session_by_refresh(db, &current_hash)
        .await?
        .ok_or_else(ApiError::unauthorized)?;

    if session.revoked_at.is_some() || session.expires_at <= Utc::now() {
        return Err(ApiError::unauthorized());
    }

    let user = store::find_user_by_id(db, session.user_id)
        .await?
        .ok_or_else(ApiError::unauthorized)?;

    ensure_active(user.status)?;

    let next_refresh = token::new_refresh_token();
    let next_hash = token::hash_refresh_token(&next_refresh);
    let expires_at = Utc::now() + Duration::seconds(live.refresh_ttl_secs);
    store::rotate_session(db, session.id, &next_hash, expires_at).await?;

    let access_token = token::issue_access(
        &user.id.to_string(),
        &session.id.to_string(),
        &config.jwt_secret,
        live.access_ttl_secs,
    )
    .map_err(|err| {
        tracing::error!(error = %err, "签发访问令牌失败");
        ApiError::internal("签发令牌失败")
    })?;

    Ok(AuthResponse {
        access_token,
        refresh_token: next_refresh,
        expires_in: live.access_ttl_secs,
        user: user.into(),
    })
}

pub async fn logout(db: &PgPool, session_id: Uuid) -> Result<(), ApiError> {
    store::revoke_session(db, session_id).await?;
    Ok(())
}

pub async fn profile(db: &PgPool, user_id: Uuid) -> Result<UserInfo, ApiError> {
    let user = store::find_user_by_id(db, user_id)
        .await?
        .ok_or_else(ApiError::unauthorized)?;

    Ok(user.into())
}

async fn issue_tokens(
    db: &PgPool,
    config: &Config,
    live: &LiveSettings,
    user_id: Uuid,
    meta: &RequestMeta,
) -> Result<(String, String), ApiError> {
    let refresh_token = token::new_refresh_token();
    let refresh_hash = token::hash_refresh_token(&refresh_token);
    let expires_at = Utc::now() + Duration::seconds(live.refresh_ttl_secs);

    let session_id = store::insert_session(db, user_id, &refresh_hash, expires_at, meta).await?;

    let access_token = token::issue_access(
        &user_id.to_string(),
        &session_id.to_string(),
        &config.jwt_secret,
        live.access_ttl_secs,
    )
    .map_err(|err| {
        tracing::error!(error = %err, "签发访问令牌失败");
        ApiError::internal("签发令牌失败")
    })?;

    Ok((access_token, refresh_token))
}

async fn ensure_phone_available(db: &PgPool, phone: Option<&str>) -> Result<(), ApiError> {
    let Some(phone) = phone else {
        return Ok(());
    };

    if store::find_user_by_phone(db, phone).await?.is_some() {
        return Err(ApiError::conflict(20001, "该手机号已注册"));
    }

    Ok(())
}

async fn ensure_email_available(db: &PgPool, email: Option<&str>) -> Result<(), ApiError> {
    let Some(email) = email else {
        return Ok(());
    };

    if store::find_user_by_email(db, email).await?.is_some() {
        return Err(ApiError::conflict(20001, "该邮箱已注册"));
    }

    Ok(())
}

fn ensure_active(status: i16) -> Result<(), ApiError> {
    match status {
        1 => Ok(()),
        2 => Err(ApiError::forbidden("账号已被禁言，暂时无法登录")),
        3 => Err(ApiError::forbidden("账号已被封禁")),
        _ => Err(ApiError::forbidden("账号不可用")),
    }
}

/// 对外展示的短编号，去掉容易看错的 0/O/1/I。
fn new_short_code() -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let raw = Uuid::new_v4().into_bytes();
    raw.iter()
        .take(10)
        .map(|byte| ALPHABET[*byte as usize % ALPHABET.len()] as char)
        .collect()
}

fn normalize_phone(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() != 11 || !trimmed.starts_with('1') {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(trimmed.to_owned())
}

fn normalize_email(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_lowercase();
    let mut parts = trimmed.split('@');
    let local = parts.next()?;
    let domain = parts.next()?;
    if parts.next().is_some() || local.is_empty() || !domain.contains('.') {
        return None;
    }
    Some(trimmed)
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.chars().count() < 8 || password.chars().count() > 64 {
        return Err(ApiError::bad_request("密码长度需要在 8 到 64 位之间"));
    }

    let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_letter || !has_digit {
        return Err(ApiError::bad_request("密码需要同时包含字母和数字"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_must_be_eleven_digits() {
        assert_eq!(
            normalize_phone("13800138000").as_deref(),
            Some("13800138000")
        );
        assert_eq!(
            normalize_phone(" 13800138000 ").as_deref(),
            Some("13800138000")
        );
        assert!(normalize_phone("23800138000").is_none());
        assert!(normalize_phone("1380013800").is_none());
        assert!(normalize_phone("1380013800a").is_none());
    }

    #[test]
    fn email_is_lowercased_and_simple_checked() {
        assert_eq!(normalize_email(" A@B.com ").as_deref(), Some("a@b.com"));
        assert!(normalize_email("a@b").is_none());
        assert!(normalize_email("a@@b.com").is_none());
        assert!(normalize_email("@b.com").is_none());
    }

    #[test]
    fn password_needs_letter_and_digit() {
        let good = format!("{}{}", "Abc", "12345");
        assert!(validate_password(&good).is_ok());
        assert!(validate_password(&"a".repeat(8)).is_err());
        assert!(validate_password(&"1".repeat(8)).is_err());
        assert!(validate_password("Ab1").is_err());
    }

    #[test]
    fn short_code_is_unique_enough() {
        let a = new_short_code();
        let b = new_short_code();
        assert_eq!(a.chars().count(), 10);
        assert_ne!(a, b);
    }
}
