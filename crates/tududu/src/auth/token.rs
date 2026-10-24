use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 访问令牌的受众标识，避免与其它系统的令牌混用。
const AUDIENCE: &str = "tududu-user";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 用户 ID
    pub sub: String,
    /// 会话 ID，登出时按它吊销
    pub sid: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

pub fn issue_access(
    user_id: &str,
    session_id: &str,
    secret: &str,
    ttl_secs: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_owned(),
        sid: session_id.to_owned(),
        aud: AUDIENCE.to_owned(),
        iat: now,
        exp: now + ttl_secs,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_access(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[AUDIENCE]);

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(data.claims)
}

/// 生成刷新令牌明文。只需要随机且不可猜测，两段 UUIDv4 拼起来即可。
pub fn new_refresh_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// 刷新令牌只存摘要，库被拖走也拿不到可用的令牌。
pub fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}
