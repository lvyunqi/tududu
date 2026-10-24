use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::RequestMeta;

/// 用户的只读投影，用于登录、鉴权和资料接口。
#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub short_code: String,
    pub nickname: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
}

const USER_COLUMNS: &str =
    "id, short_code, nickname, phone, email, password_hash, status, created_at";

pub async fn find_user_by_phone(db: &PgPool, phone: &str) -> sqlx::Result<Option<UserRow>> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE phone = $1");
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(phone)
        .fetch_optional(db)
        .await
}

pub async fn find_user_by_email(db: &PgPool, email: &str) -> sqlx::Result<Option<UserRow>> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE email = $1");
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(email)
        .fetch_optional(db)
        .await
}

pub async fn find_user_by_id(db: &PgPool, id: Uuid) -> sqlx::Result<Option<UserRow>> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1");
    sqlx::query_as::<_, UserRow>(&sql)
        .bind(id)
        .fetch_optional(db)
        .await
}

pub struct NewUser<'a> {
    pub short_code: &'a str,
    pub nickname: &'a str,
    pub phone: Option<&'a str>,
    pub email: Option<&'a str>,
    pub password_hash: &'a str,
    pub register_ip: Option<&'a str>,
}

pub async fn insert_user(db: &PgPool, user: &NewUser<'_>) -> sqlx::Result<UserRow> {
    let sql = format!(
        "INSERT INTO users (short_code, nickname, phone, email, password_hash, register_channel, register_ip) \
         VALUES ($1, $2, $3, $4, $5, 'app', $6) \
         RETURNING {USER_COLUMNS}"
    );

    sqlx::query_as::<_, UserRow>(&sql)
        .bind(user.short_code)
        .bind(user.nickname)
        .bind(user.phone)
        .bind(user.email)
        .bind(user.password_hash)
        .bind(user.register_ip)
        .fetch_one(db)
        .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn insert_session(
    db: &PgPool,
    user_id: Uuid,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
    meta: &RequestMeta,
) -> sqlx::Result<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO user_sessions \
           (user_id, device_id, device_name, platform, app_version, refresh_hash, ip, user_agent, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id"
    )
    .bind(user_id)
    .bind(meta.device_id.clone())
    .bind(meta.device_name.clone())
    .bind(meta.platform.clone())
    .bind(meta.app_version.clone())
    .bind(refresh_hash)
    .bind(meta.ip.clone())
    .bind(meta.user_agent.clone())
    .bind(expires_at)
    .fetch_one(db)
    .await?;

    Ok(row.0)
}

pub async fn find_session_by_refresh(
    db: &PgPool,
    refresh_hash: &str,
) -> sqlx::Result<Option<SessionRow>> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, expires_at, revoked_at FROM user_sessions WHERE refresh_hash = $1",
    )
    .bind(refresh_hash)
    .fetch_optional(db)
    .await
}

pub async fn rotate_session(
    db: &PgPool,
    session_id: Uuid,
    refresh_hash: &str,
    expires_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE user_sessions \
            SET refresh_hash = $2, expires_at = $3, last_used_at = now() \
          WHERE id = $1",
    )
    .bind(session_id)
    .bind(refresh_hash)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn revoke_session(db: &PgPool, session_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE user_sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(session_id)
        .execute(db)
        .await?;

    Ok(())
}

/// 唯一键冲突，用来区分「手机号已注册」和真正的故障。
pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}
