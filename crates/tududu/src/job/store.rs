use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// 任务状态。
pub const STATUS_QUEUED: i16 = 0;
pub const STATUS_RUNNING: i16 = 1;
pub const STATUS_SUCCEEDED: i16 = 2;
pub const STATUS_FAILED: i16 = 3;
pub const STATUS_CANCELED: i16 = 4;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct JobRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub model: String,
    pub prompt: String,
    pub input_asset_id: Option<Uuid>,
    pub status: i16,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub output_asset_id: Option<Uuid>,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

const JOB_COLUMNS: &str = "id, user_id, provider_id, channel_id, model, prompt, input_asset_id, status, attempt_count, max_attempts, output_asset_id, error_code, error_message, started_at, finished_at, created_at";

pub struct NewJob<'a> {
    pub user_id: Uuid,
    pub provider_id: Option<Uuid>,
    pub model: &'a str,
    pub prompt: &'a str,
    pub input_asset_id: Option<Uuid>,
    pub max_attempts: i32,
}

pub async fn insert(db: &PgPool, item: &NewJob<'_>) -> sqlx::Result<JobRow> {
    let sql = format!(
        "INSERT INTO jobs (user_id, provider_id, model, prompt, input_asset_id, max_attempts) VALUES ($1, $2, $3, $4, $5, $6) RETURNING {JOB_COLUMNS}"
    );

    sqlx::query_as::<_, JobRow>(&sql)
        .bind(item.user_id)
        .bind(item.provider_id)
        .bind(item.model)
        .bind(item.prompt)
        .bind(item.input_asset_id)
        .bind(item.max_attempts)
        .fetch_one(db)
        .await
}

pub async fn find_owned(db: &PgPool, id: Uuid, owner: Uuid) -> sqlx::Result<Option<JobRow>> {
    let sql = format!("SELECT {JOB_COLUMNS} FROM jobs WHERE id = $1 AND user_id = $2");

    sqlx::query_as::<_, JobRow>(&sql)
        .bind(id)
        .bind(owner)
        .fetch_optional(db)
        .await
}

pub async fn find(db: &PgPool, id: Uuid) -> sqlx::Result<Option<JobRow>> {
    let sql = format!("SELECT {JOB_COLUMNS} FROM jobs WHERE id = $1");

    sqlx::query_as::<_, JobRow>(&sql)
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn mark_running(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE jobs SET status = $2, started_at = now() WHERE id = $1 AND status = $3")
        .bind(id)
        .bind(STATUS_RUNNING)
        .bind(STATUS_QUEUED)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn bump_attempt(
    db: &PgPool,
    id: Uuid,
    attempt_no: i32,
    channel_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE jobs SET attempt_count = $2, channel_id = $3 WHERE id = $1")
        .bind(id)
        .bind(attempt_no)
        .bind(channel_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn mark_succeeded(db: &PgPool, id: Uuid, output_asset_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE jobs SET status = $2, output_asset_id = $3, finished_at = now() WHERE id = $1")
        .bind(id)
        .bind(STATUS_SUCCEEDED)
        .bind(output_asset_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn mark_failed(
    db: &PgPool,
    id: Uuid,
    error_code: i32,
    error_message: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE jobs SET status = $2, error_code = $3, error_message = $4, finished_at = now() WHERE id = $1")
        .bind(id)
        .bind(STATUS_FAILED)
        .bind(error_code)
        .bind(error_message)
        .execute(db)
        .await?;

    Ok(())
}

pub struct NewAttempt<'a> {
    pub job_id: Uuid,
    pub attempt_no: i32,
    pub channel_id: Uuid,
    pub status: i16,
    pub http_status: Option<i32>,
    pub latency_ms: i32,
    pub error_code: Option<&'a str>,
    pub error_body: Option<&'a str>,
}

pub async fn insert_attempt(db: &PgPool, item: &NewAttempt<'_>) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO job_attempts (job_id, attempt_no, channel_id, status, http_status, latency_ms, error_code, error_body) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(item.job_id)
        .bind(item.attempt_no)
        .bind(item.channel_id)
        .bind(item.status)
        .bind(item.http_status)
        .bind(item.latency_ms)
        .bind(item.error_code)
        .bind(item.error_body)
        .execute(db)
        .await?;

    Ok(())
}

/// 任务尝试的状态：1 成功 / 2 可重试失败 / 3 不可重试失败。
pub const ATTEMPT_OK: i16 = 1;
pub const ATTEMPT_RETRYABLE: i16 = 2;
pub const ATTEMPT_FATAL: i16 = 3;
