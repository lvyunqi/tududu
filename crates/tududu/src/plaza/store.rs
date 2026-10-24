use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub const STATUS_PUBLISHED: i16 = 2;
pub const STATUS_HIDDEN: i16 = 3;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PromptRow {
    pub id: Uuid,
    pub short_code: String,
    pub author_user_id: Uuid,
    pub title: String,
    pub prompt_text: String,
    pub model: String,
    pub category_id: Option<i64>,
    pub cover_asset_id: Option<Uuid>,
    pub like_count: i32,
    pub view_count: i32,
    pub published_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

const PROMPT_COLUMNS: &str = "id, short_code, author_user_id, title, prompt_text, model, category_id, cover_asset_id, like_count, view_count, published_at, created_at";

pub async fn list_categories(db: &PgPool) -> sqlx::Result<Vec<CategoryRow>> {
    sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name, slug FROM prompt_categories WHERE enabled ORDER BY sort, id",
    )
    .fetch_all(db)
    .await
}

pub struct NewPrompt<'a> {
    pub short_code: &'a str,
    pub author_user_id: Uuid,
    pub title: &'a str,
    pub prompt_text: &'a str,
    pub model: &'a str,
    pub category_id: Option<i64>,
    pub cover_asset_id: Option<Uuid>,
    pub source_session_id: Option<Uuid>,
}

pub async fn insert(db: &PgPool, item: &NewPrompt<'_>) -> sqlx::Result<PromptRow> {
    let sql = format!(
        "INSERT INTO prompts (short_code, author_user_id, title, prompt_text, model, category_id, cover_asset_id, source_session_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {PROMPT_COLUMNS}"
    );

    sqlx::query_as::<_, PromptRow>(&sql)
        .bind(item.short_code)
        .bind(item.author_user_id)
        .bind(item.title)
        .bind(item.prompt_text)
        .bind(item.model)
        .bind(item.category_id)
        .bind(item.cover_asset_id)
        .bind(item.source_session_id)
        .fetch_one(db)
        .await
}

pub async fn find_published(db: &PgPool, code: &str) -> sqlx::Result<Option<PromptRow>> {
    let sql = format!("SELECT {PROMPT_COLUMNS} FROM prompts WHERE short_code = $1 AND status = 2");

    sqlx::query_as::<_, PromptRow>(&sql)
        .bind(code)
        .fetch_optional(db)
        .await
}

pub async fn list_published(
    db: &PgPool,
    category_id: Option<i64>,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<PromptRow>> {
    let sql = format!(
        "SELECT {PROMPT_COLUMNS} FROM prompts WHERE status = 2 AND ($1::bigint IS NULL OR category_id = $1) AND ($2::timestamptz IS NULL OR published_at < $2) ORDER BY published_at DESC LIMIT $3"
    );

    sqlx::query_as::<_, PromptRow>(&sql)
        .bind(category_id)
        .bind(before)
        .bind(limit)
        .fetch_all(db)
        .await
}

pub async fn list_by_author(
    db: &PgPool,
    author: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<PromptRow>> {
    let sql = format!(
        "SELECT {PROMPT_COLUMNS} FROM prompts WHERE author_user_id = $1 AND status <> 3 AND ($2::timestamptz IS NULL OR created_at < $2) ORDER BY created_at DESC LIMIT $3"
    );

    sqlx::query_as::<_, PromptRow>(&sql)
        .bind(author)
        .bind(before)
        .bind(limit)
        .fetch_all(db)
        .await
}

pub async fn bump_view(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE prompts SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    Ok(())
}

/// 切换点赞。返回 true 表示这次是点赞，false 表示取消。
pub async fn toggle_like(db: &PgPool, user_id: Uuid, prompt_id: Uuid) -> sqlx::Result<bool> {
    let inserted = sqlx::query(
        "INSERT INTO prompt_likes (user_id, prompt_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(prompt_id)
    .execute(db)
    .await?;

    if inserted.rows_affected() > 0 {
        return Ok(true);
    }

    sqlx::query("DELETE FROM prompt_likes WHERE user_id = $1 AND prompt_id = $2")
        .bind(user_id)
        .bind(prompt_id)
        .execute(db)
        .await?;

    Ok(false)
}

pub async fn adjust_like_count(db: &PgPool, prompt_id: Uuid, delta: i32) -> sqlx::Result<i32> {
    sqlx::query_scalar(
        "UPDATE prompts SET like_count = GREATEST(like_count + $2, 0) WHERE id = $1 RETURNING like_count",
    )
    .bind(prompt_id)
    .bind(delta)
    .fetch_one(db)
    .await
}

pub async fn list_by_status(
    db: &PgPool,
    status: i16,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<PromptRow>> {
    let sql = format!(
        "SELECT {PROMPT_COLUMNS} FROM prompts WHERE status = $1 AND ($2::timestamptz IS NULL OR created_at < $2) ORDER BY created_at DESC LIMIT $3"
    );

    sqlx::query_as::<_, PromptRow>(&sql)
        .bind(status)
        .bind(before)
        .bind(limit)
        .fetch_all(db)
        .await
}

/// 管理端直接改状态，用于下架或恢复。
pub async fn set_status(db: &PgPool, code: &str, status: i16) -> sqlx::Result<bool> {
    let result = sqlx::query("UPDATE prompts SET status = $2 WHERE short_code = $1")
        .bind(code)
        .bind(status)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn hide(db: &PgPool, code: &str, author: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE prompts SET status = 3 WHERE short_code = $1 AND author_user_id = $2 AND status = 2",
    )
    .bind(code)
    .bind(author)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}
