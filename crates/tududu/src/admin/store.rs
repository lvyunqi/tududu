use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub short_code: String,
    pub nickname: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: i16,
    pub balance: i64,
    pub used_bytes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminOrderRow {
    pub order_no: String,
    pub user_id: Uuid,
    pub amount_fen: i64,
    pub credit: i64,
    pub bonus: i64,
    pub status: i16,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminJobRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub model: String,
    pub status: i16,
    pub price: i64,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AttemptRow {
    pub attempt_no: i32,
    pub channel_id: Option<Uuid>,
    pub status: i16,
    pub http_status: Option<i32>,
    pub latency_ms: Option<i32>,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OverviewRow {
    pub users_total: i64,
    pub users_today: i64,
    pub paid_orders_today: i64,
    pub revenue_fen_today: i64,
    pub jobs_today: i64,
    pub jobs_failed_today: i64,
    pub prompts_published: i64,
}

pub async fn list_users(
    db: &PgPool,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<AdminUserRow>> {
    sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.short_code, u.nickname, u.phone, u.email, u.status, \
                COALESCE(w.balance, 0) AS balance, \
                COALESCE((SELECT SUM(a.byte_size) FROM assets a \
                           WHERE a.owner_user_id = u.id AND a.status = 1), 0)::bigint AS used_bytes, \
                u.created_at \
           FROM users u LEFT JOIN wallets w ON w.user_id = u.id \
          WHERE ($1::timestamptz IS NULL OR u.created_at < $1) \
          ORDER BY u.created_at DESC LIMIT $2",
    )
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn set_user_status(db: &PgPool, id: Uuid, status: i16) -> sqlx::Result<bool> {
    let result = sqlx::query("UPDATE users SET status = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(db)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_orders(
    db: &PgPool,
    status: Option<i16>,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<AdminOrderRow>> {
    sqlx::query_as::<_, AdminOrderRow>(
        "SELECT order_no, user_id, amount_fen, credit, bonus, status, paid_at, created_at \
           FROM pay_orders \
          WHERE ($1::smallint IS NULL OR status = $1) \
            AND ($2::timestamptz IS NULL OR created_at < $2) \
          ORDER BY created_at DESC LIMIT $3",
    )
    .bind(status)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn list_jobs(
    db: &PgPool,
    status: Option<i16>,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<AdminJobRow>> {
    sqlx::query_as::<_, AdminJobRow>(
        "SELECT id, user_id, model, status, price, error_code, error_message, finished_at, created_at \
           FROM jobs \
          WHERE ($1::smallint IS NULL OR status = $1) \
            AND ($2::timestamptz IS NULL OR created_at < $2) \
          ORDER BY created_at DESC LIMIT $3",
    )
    .bind(status)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn find_job(db: &PgPool, id: Uuid) -> sqlx::Result<Option<AdminJobRow>> {
    sqlx::query_as::<_, AdminJobRow>(
        "SELECT id, user_id, model, status, price, error_code, error_message, finished_at, created_at FROM jobs WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn list_attempts(db: &PgPool, job_id: Uuid) -> sqlx::Result<Vec<AttemptRow>> {
    sqlx::query_as::<_, AttemptRow>(
        "SELECT attempt_no, channel_id, status, http_status, latency_ms, error_code, created_at \
           FROM job_attempts WHERE job_id = $1 ORDER BY attempt_no",
    )
    .bind(job_id)
    .fetch_all(db)
    .await
}

pub async fn overview(db: &PgPool) -> sqlx::Result<OverviewRow> {
    sqlx::query_as::<_, OverviewRow>(
        "SELECT
            (SELECT count(*) FROM users) AS users_total,
            (SELECT count(*) FROM users WHERE created_at >= date_trunc('day', now())) AS users_today,
            (SELECT count(*) FROM pay_orders WHERE status = 1 AND paid_at >= date_trunc('day', now())) AS paid_orders_today,
            -- sum() 在 Postgres 里返回 numeric，这里必须转成 bigint，否则按 i64 解码会失败
            (SELECT COALESCE(sum(amount_fen), 0)::bigint FROM pay_orders WHERE status = 1 AND paid_at >= date_trunc('day', now())) AS revenue_fen_today,
            (SELECT count(*) FROM jobs WHERE created_at >= date_trunc('day', now())) AS jobs_today,
            (SELECT count(*) FROM jobs WHERE status = 3 AND created_at >= date_trunc('day', now())) AS jobs_failed_today,
            (SELECT count(*) FROM prompts WHERE status = 2) AS prompts_published"
    )
    .fetch_one(db)
    .await
}

pub async fn record_audit(
    db: &PgPool,
    operator_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: &str,
    detail: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs (operator_id, action, target_type, target_id, detail) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(operator_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(detail)
    .execute(db)
    .await?;

    Ok(())
}
