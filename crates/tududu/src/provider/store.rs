use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

const PROVIDER_COLUMNS: &str = "id, code, name, protocol, base_url, auth_type, timeout_ms, max_concurrency, enabled, created_at";

const CHANNEL_COLUMNS: &str = "id, provider_id, name, api_key_hint, weight, priority, max_concurrency, health, enabled, created_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProviderRow {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub protocol: String,
    pub base_url: String,
    pub auth_type: String,
    pub timeout_ms: i32,
    pub max_concurrency: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChannelRow {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub name: String,
    pub api_key_hint: String,
    pub weight: i32,
    pub priority: i16,
    pub max_concurrency: i32,
    pub health: i16,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ModelRow {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub code: String,
    pub display_name: String,
    pub enabled: bool,
    pub discovered_at: DateTime<Utc>,
}

pub async fn list_providers(db: &PgPool) -> sqlx::Result<Vec<ProviderRow>> {
    let sql = format!("SELECT {PROVIDER_COLUMNS} FROM providers ORDER BY created_at");
    sqlx::query_as::<_, ProviderRow>(&sql).fetch_all(db).await
}

pub async fn find_provider(db: &PgPool, id: Uuid) -> sqlx::Result<Option<ProviderRow>> {
    let sql = format!("SELECT {PROVIDER_COLUMNS} FROM providers WHERE id = $1");

    sqlx::query_as::<_, ProviderRow>(&sql)
        .bind(id)
        .fetch_optional(db)
        .await
}

pub struct NewProvider<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub protocol: &'a str,
    pub base_url: &'a str,
    pub timeout_ms: i32,
    pub max_concurrency: i32,
}

pub async fn insert_provider(db: &PgPool, item: &NewProvider<'_>) -> sqlx::Result<ProviderRow> {
    let sql = format!(
        "INSERT INTO providers (code, name, protocol, base_url, timeout_ms, max_concurrency) VALUES ($1, $2, $3, $4, $5, $6) RETURNING {PROVIDER_COLUMNS}"
    );

    sqlx::query_as::<_, ProviderRow>(&sql)
        .bind(item.code)
        .bind(item.name)
        .bind(item.protocol)
        .bind(item.base_url)
        .bind(item.timeout_ms)
        .bind(item.max_concurrency)
        .fetch_one(db)
        .await
}

pub async fn update_provider(
    db: &PgPool,
    id: Uuid,
    name: Option<&str>,
    base_url: Option<&str>,
    timeout_ms: Option<i32>,
    max_concurrency: Option<i32>,
    enabled: Option<bool>,
) -> sqlx::Result<Option<ProviderRow>> {
    let sql = format!(
        "UPDATE providers SET name = COALESCE($2, name), base_url = COALESCE($3, base_url), timeout_ms = COALESCE($4, timeout_ms), max_concurrency = COALESCE($5, max_concurrency), enabled = COALESCE($6, enabled), updated_at = now() WHERE id = $1 RETURNING {PROVIDER_COLUMNS}"
    );

    sqlx::query_as::<_, ProviderRow>(&sql)
        .bind(id)
        .bind(name)
        .bind(base_url)
        .bind(timeout_ms)
        .bind(max_concurrency)
        .bind(enabled)
        .fetch_optional(db)
        .await
}

pub async fn delete_provider(db: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM providers WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub struct NewChannel<'a> {
    pub provider_id: Uuid,
    pub name: &'a str,
    pub api_key_cipher: &'a str,
    pub api_key_hint: &'a str,
    pub weight: i32,
    pub priority: i16,
    pub max_concurrency: i32,
}

pub async fn insert_channel(db: &PgPool, item: &NewChannel<'_>) -> sqlx::Result<ChannelRow> {
    let sql = format!(
        "INSERT INTO channels (provider_id, name, api_key_cipher, api_key_hint, weight, priority, max_concurrency) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {CHANNEL_COLUMNS}"
    );

    sqlx::query_as::<_, ChannelRow>(&sql)
        .bind(item.provider_id)
        .bind(item.name)
        .bind(item.api_key_cipher)
        .bind(item.api_key_hint)
        .bind(item.weight)
        .bind(item.priority)
        .bind(item.max_concurrency)
        .fetch_one(db)
        .await
}

pub async fn list_channels(db: &PgPool, provider_id: Uuid) -> sqlx::Result<Vec<ChannelRow>> {
    let sql = format!(
        "SELECT {CHANNEL_COLUMNS} FROM channels WHERE provider_id = $1 ORDER BY priority, created_at"
    );

    sqlx::query_as::<_, ChannelRow>(&sql)
        .bind(provider_id)
        .fetch_all(db)
        .await
}

pub async fn find_channel_secret(db: &PgPool, id: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT api_key_cipher FROM channels WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn upsert_models(
    db: &PgPool,
    provider_id: Uuid,
    codes: &[String],
) -> sqlx::Result<Vec<ModelRow>> {
    for code in codes {
        sqlx::query("INSERT INTO models (provider_id, code, display_name) VALUES ($1, $2, $2) ON CONFLICT (provider_id, code) DO UPDATE SET discovered_at = now()")
            .bind(provider_id)
            .bind(code)
            .execute(db)
            .await?;
    }

    list_models(db, provider_id).await
}

/// 唯一键冲突，用来区分「标识已存在」和真正的故障。
pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

pub async fn list_models(db: &PgPool, provider_id: Uuid) -> sqlx::Result<Vec<ModelRow>> {
    sqlx::query_as::<_, ModelRow>("SELECT id, provider_id, code, display_name, enabled, discovered_at FROM models WHERE provider_id = $1 ORDER BY code")
        .bind(provider_id)
        .fetch_all(db)
        .await
}
