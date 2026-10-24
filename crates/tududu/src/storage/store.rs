use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AssetRow {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    #[serde(skip)]
    pub object_key: String,
    #[serde(skip)]
    pub thumb_key: Option<String>,
    pub mime: String,
    pub byte_size: i64,
    pub sha256: String,
    pub source: String,
    pub status: i16,
    pub created_at: DateTime<Utc>,
}

const ASSET_COLUMNS: &str =
    "id, owner_user_id, object_key, thumb_key, mime, byte_size, sha256, source, status, created_at";

pub async fn find_by_sha(db: &PgPool, owner: Uuid, sha256: &str) -> sqlx::Result<Option<AssetRow>> {
    let sql = format!(
        "SELECT {ASSET_COLUMNS} FROM assets WHERE owner_user_id = $1 AND sha256 = $2 AND status = 1"
    );

    sqlx::query_as::<_, AssetRow>(&sql)
        .bind(owner)
        .bind(sha256)
        .fetch_optional(db)
        .await
}

pub async fn find_owned(db: &PgPool, id: Uuid, owner: Uuid) -> sqlx::Result<Option<AssetRow>> {
    let sql = format!(
        "SELECT {ASSET_COLUMNS} FROM assets WHERE id = $1 AND owner_user_id = $2 AND status = 1"
    );

    sqlx::query_as::<_, AssetRow>(&sql)
        .bind(id)
        .bind(owner)
        .fetch_optional(db)
        .await
}

pub struct NewAsset<'a> {
    pub owner_user_id: Uuid,
    pub object_key: &'a str,
    pub thumb_key: Option<&'a str>,
    pub mime: &'a str,
    pub byte_size: i64,
    pub sha256: &'a str,
    pub source: &'a str,
}

pub async fn insert(db: &PgPool, item: &NewAsset<'_>) -> sqlx::Result<AssetRow> {
    let sql = format!(
        "INSERT INTO assets (owner_user_id, object_key, thumb_key, mime, byte_size, sha256, source) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {ASSET_COLUMNS}"
    );

    sqlx::query_as::<_, AssetRow>(&sql)
        .bind(item.owner_user_id)
        .bind(item.object_key)
        .bind(item.thumb_key)
        .bind(item.mime)
        .bind(item.byte_size)
        .bind(item.sha256)
        .bind(item.source)
        .fetch_one(db)
        .await
}

pub async fn list(
    db: &PgPool,
    owner: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<AssetRow>> {
    let sql = format!(
        "SELECT {ASSET_COLUMNS} FROM assets WHERE owner_user_id = $1 AND status = 1 AND ($2::timestamptz IS NULL OR created_at < $2) ORDER BY created_at DESC LIMIT $3"
    );

    sqlx::query_as::<_, AssetRow>(&sql)
        .bind(owner)
        .bind(before)
        .bind(limit)
        .fetch_all(db)
        .await
}

pub async fn soft_delete(db: &PgPool, id: Uuid, owner: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE assets SET status = 2 WHERE id = $1 AND owner_user_id = $2 AND status = 1",
    )
    .bind(id)
    .bind(owner)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}
