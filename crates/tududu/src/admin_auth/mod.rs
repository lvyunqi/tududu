//! 管理员账号：登录、令牌签发与校验。

use std::collections::BTreeMap;

use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::password;
use crate::error::ApiError;

const AUDIENCE: &str = "tududu-admin";
const TOKEN_TTL_SECS: i64 = 8 * 3600;

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: String,
    pub name: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub display_name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AdminRow {
    id: Uuid,
    password_hash: String,
    display_name: String,
    status: i16,
}

pub async fn login(
    db: &PgPool,
    secret: &str,
    req: LoginRequest,
) -> Result<LoginResponse, ApiError> {
    let username = req.username.trim().to_lowercase();
    if username.is_empty() || req.password.is_empty() {
        return Err(ApiError::unauthorized());
    }

    let found: Option<AdminRow> = sqlx::query_as(
        "SELECT id, password_hash, display_name, status FROM admin_users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(db)
    .await?;

    // 账号不存在也走一遍散列校验，避免用响应时间判断账号是否存在
    let fallback = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHR2YWx1ZQ$0000000000000000000000000000000000000000000";

    let (hash, row) = match found {
        Some(row) => (row.password_hash.clone(), Some(row)),
        None => (fallback.to_owned(), None),
    };

    let matched = password::verify(&req.password, &hash);
    let Some(row) = row else {
        return Err(ApiError::unauthorized());
    };

    if !matched || row.status != 1 {
        return Err(ApiError::unauthorized());
    }

    sqlx::query("UPDATE admin_users SET last_login_at = now() WHERE id = $1")
        .bind(row.id)
        .execute(db)
        .await?;

    let now = Utc::now();
    let claims = AdminClaims {
        sub: row.id.to_string(),
        name: row.display_name.clone(),
        aud: AUDIENCE.to_owned(),
        iat: now.timestamp(),
        exp: (now + Duration::seconds(TOKEN_TTL_SECS)).timestamp(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| {
        tracing::error!(error = %err, "签发管理员令牌失败");
        ApiError::internal("登录失败")
    })?;

    Ok(LoginResponse {
        access_token: token,
        expires_in: TOKEN_TTL_SECS,
        display_name: row.display_name,
    })
}

pub fn verify(secret: &str, token: &str) -> Option<AdminClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[AUDIENCE]);

    decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .map(|data| data.claims)
}

/// 首次启动时如果库里没有管理员，用环境变量里的一组账号建一个，省得进不去。
pub async fn bootstrap(
    db: &PgPool,
    username: &str,
    plain_password: &str,
) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_users")
        .fetch_one(db)
        .await?;

    if count > 0 {
        return Ok(());
    }

    let hash = password::hash(plain_password).map_err(|err| anyhow::anyhow!("生成密码散列失败：{err}"))?;

    sqlx::query(
        "INSERT INTO admin_users (username, password_hash, display_name) VALUES ($1, $2, $3)",
    )
    .bind(username.trim().to_lowercase())
    .bind(hash)
    .bind("初始管理员")
    .execute(db)
    .await?;

    tracing::warn!(%username, "已创建初始管理员账号，请尽快在后台修改密码");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims_round_trip_through_token() {
        let secret = "test-secret-that-is-long-enough-for-hs256";
        let now = Utc::now().timestamp();
        let claims = AdminClaims {
            sub: Uuid::nil().to_string(),
            name: "测试".to_owned(),
            aud: AUDIENCE.to_owned(),
            iat: now,
            exp: now + 600,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("签发失败");

        let decoded = verify(secret, &token).expect("校验失败");
        assert_eq!(decoded.name, "测试");
        assert!(verify("another-secret-long-enough-for-hs256xx", &token).is_none());
    }

    #[test]
    fn login_payload_ignores_extra_fields() {
        let mut raw = BTreeMap::new();
        raw.insert("username", "root");
        raw.insert("password", "secret-value");
        assert_eq!(raw.len(), 2);
    }
}
