use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub const STATUS_NORMAL: i16 = 1;

pub const MSG_PENDING: i16 = 1;
pub const MSG_DONE: i16 = 3;
pub const MSG_FAILED: i16 = 4;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub title: String,
    pub model: String,
    pub status: i16,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MessageRow {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub input_asset_id: Option<Uuid>,
    pub output_asset_id: Option<Uuid>,
    pub job_id: Option<Uuid>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
}

const SESSION_COLUMNS: &str = "id, title, model, status, last_active_at, created_at";

const MESSAGE_COLUMNS: &str =
    "id, role, content, input_asset_id, output_asset_id, job_id, status, created_at";

pub async fn insert(
    db: &PgPool,
    user_id: Uuid,
    title: &str,
    model: &str,
) -> sqlx::Result<SessionRow> {
    let sql = format!(
        "INSERT INTO sessions (user_id, title, model) VALUES ($1, $2, $3) RETURNING {SESSION_COLUMNS}"
    );

    sqlx::query_as::<_, SessionRow>(&sql)
        .bind(user_id)
        .bind(title)
        .bind(model)
        .fetch_one(db)
        .await
}

pub async fn find_owned(db: &PgPool, id: Uuid, user_id: Uuid) -> sqlx::Result<Option<SessionRow>> {
    let sql = format!(
        "SELECT {SESSION_COLUMNS} FROM sessions WHERE id = $1 AND user_id = $2 AND status <> 3"
    );

    sqlx::query_as::<_, SessionRow>(&sql)
        .bind(id)
        .bind(user_id)
        .fetch_optional(db)
        .await
}

pub async fn list(
    db: &PgPool,
    user_id: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<SessionRow>> {
    let sql = format!(
        "SELECT {SESSION_COLUMNS} FROM sessions WHERE user_id = $1 AND status <> 3 AND ($2::timestamptz IS NULL OR last_active_at < $2) ORDER BY last_active_at DESC LIMIT $3"
    );

    sqlx::query_as::<_, SessionRow>(&sql)
        .bind(user_id)
        .bind(before)
        .bind(limit)
        .fetch_all(db)
        .await
}

pub async fn rename(db: &PgPool, id: Uuid, user_id: Uuid, title: &str) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE sessions SET title = $3 WHERE id = $1 AND user_id = $2 AND status <> 3",
    )
    .bind(id)
    .bind(user_id)
    .bind(title)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn soft_delete(db: &PgPool, id: Uuid, user_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE sessions SET status = 3 WHERE id = $1 AND user_id = $2 AND status <> 3",
    )
    .bind(id)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn touch(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE sessions SET last_active_at = now() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;

    Ok(())
}

pub struct NewMessage<'a> {
    pub session_id: Uuid,
    pub role: &'a str,
    pub content: &'a str,
    pub input_asset_id: Option<Uuid>,
    pub status: i16,
}

pub async fn insert_message(db: &PgPool, item: &NewMessage<'_>) -> sqlx::Result<MessageRow> {
    let sql = format!(
        "INSERT INTO session_messages (session_id, role, content, input_asset_id, status) VALUES ($1, $2, $3, $4, $5) RETURNING {MESSAGE_COLUMNS}"
    );

    sqlx::query_as::<_, MessageRow>(&sql)
        .bind(item.session_id)
        .bind(item.role)
        .bind(item.content)
        .bind(item.input_asset_id)
        .bind(item.status)
        .fetch_one(db)
        .await
}

pub async fn list_messages(
    db: &PgPool,
    session_id: Uuid,
    limit: i64,
) -> sqlx::Result<Vec<MessageRow>> {
    let sql = format!(
        "SELECT {MESSAGE_COLUMNS} FROM session_messages WHERE session_id = $1 ORDER BY created_at LIMIT $2"
    );

    sqlx::query_as::<_, MessageRow>(&sql)
        .bind(session_id)
        .bind(limit)
        .fetch_all(db)
        .await
}

pub async fn attach_job(db: &PgPool, message_id: Uuid, job_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE session_messages SET job_id = $2 WHERE id = $1")
        .bind(message_id)
        .bind(job_id)
        .execute(db)
        .await?;

    Ok(())
}

/// 任务结束时回写助手消息，成功记结果图，失败只改状态。
pub async fn finish_message(
    db: &PgPool,
    message_id: Uuid,
    status: i16,
    output_asset_id: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE session_messages SET status = $2, output_asset_id = COALESCE($3, output_asset_id) WHERE id = $1")
        .bind(message_id)
        .bind(status)
        .bind(output_asset_id)
        .execute(db)
        .await?;

    Ok(())
}
