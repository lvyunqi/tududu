use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub const ORDER_PENDING: i16 = 0;
pub const ORDER_PAID: i16 = 1;
pub const ORDER_CLOSED: i16 = 2;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlanRow {
    pub id: Uuid,
    pub name: String,
    pub amount_fen: i64,
    pub credit: i64,
    pub bonus: i64,
    pub sort: i32,
    pub enabled: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChannelSecretRow {
    pub id: Uuid,
    pub gateway_url: String,
    pub pid: String,
    pub key_cipher: String,
    pub pay_type: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PayChannelRow {
    pub id: Uuid,
    pub name: String,
    pub gateway_url: String,
    pub pid: String,
    pub pay_type: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderRow {
    pub id: Uuid,
    pub order_no: String,
    pub user_id: Uuid,
    pub amount_fen: i64,
    pub credit: i64,
    pub bonus: i64,
    pub status: i16,
    pub subject: String,
    pub trade_no: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub expire_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

const ORDER_COLUMNS: &str = "id, order_no, user_id, amount_fen, credit, bonus, status, subject, trade_no, paid_at, expire_at, created_at";

pub async fn list_plans(db: &PgPool) -> sqlx::Result<Vec<PlanRow>> {
    sqlx::query_as::<_, PlanRow>(
        "SELECT id, name, amount_fen, credit, bonus, sort, enabled FROM recharge_plans WHERE enabled ORDER BY sort, amount_fen",
    )
    .fetch_all(db)
    .await
}

pub async fn find_plan(db: &PgPool, id: Uuid) -> sqlx::Result<Option<PlanRow>> {
    sqlx::query_as::<_, PlanRow>(
        "SELECT id, name, amount_fen, credit, bonus, sort, enabled FROM recharge_plans WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn insert_plan(
    db: &PgPool,
    name: &str,
    amount_fen: i64,
    credit: i64,
    bonus: i64,
    sort: i32,
) -> sqlx::Result<PlanRow> {
    sqlx::query_as::<_, PlanRow>(
        "INSERT INTO recharge_plans (name, amount_fen, credit, bonus, sort) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, name, amount_fen, credit, bonus, sort, enabled",
    )
    .bind(name)
    .bind(amount_fen)
    .bind(credit)
    .bind(bonus)
    .bind(sort)
    .fetch_one(db)
    .await
}

pub async fn list_channels(db: &PgPool) -> sqlx::Result<Vec<PayChannelRow>> {
    sqlx::query_as::<_, PayChannelRow>(
        "SELECT id, name, gateway_url, pid, pay_type, enabled, created_at FROM pay_channels ORDER BY created_at",
    )
    .fetch_all(db)
    .await
}

pub async fn insert_channel(
    db: &PgPool,
    name: &str,
    gateway_url: &str,
    pid: &str,
    key_cipher: &str,
    pay_type: &str,
) -> sqlx::Result<PayChannelRow> {
    sqlx::query_as::<_, PayChannelRow>(
        "INSERT INTO pay_channels (name, gateway_url, pid, key_cipher, pay_type) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, name, gateway_url, pid, pay_type, enabled, created_at",
    )
    .bind(name)
    .bind(gateway_url)
    .bind(pid)
    .bind(key_cipher)
    .bind(pay_type)
    .fetch_one(db)
    .await
}

pub async fn pick_channel(db: &PgPool) -> sqlx::Result<Option<ChannelSecretRow>> {
    sqlx::query_as::<_, ChannelSecretRow>(
        "SELECT id, gateway_url, pid, key_cipher, pay_type FROM pay_channels WHERE enabled ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(db)
    .await
}

pub async fn find_channel_by_pid(db: &PgPool, pid: &str) -> sqlx::Result<Option<ChannelSecretRow>> {
    sqlx::query_as::<_, ChannelSecretRow>(
        "SELECT id, gateway_url, pid, key_cipher, pay_type FROM pay_channels WHERE pid = $1 AND enabled LIMIT 1",
    )
    .bind(pid)
    .fetch_optional(db)
    .await
}

pub struct NewOrder<'a> {
    pub order_no: &'a str,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub channel_id: Uuid,
    pub amount_fen: i64,
    pub credit: i64,
    pub bonus: i64,
    pub subject: &'a str,
    pub expire_at: DateTime<Utc>,
}

pub async fn insert_order(db: &PgPool, item: &NewOrder<'_>) -> sqlx::Result<OrderRow> {
    let sql = format!(
        "INSERT INTO pay_orders (order_no, user_id, plan_id, channel_id, amount_fen, credit, bonus, subject, expire_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING {ORDER_COLUMNS}"
    );

    sqlx::query_as::<_, OrderRow>(&sql)
        .bind(item.order_no)
        .bind(item.user_id)
        .bind(item.plan_id)
        .bind(item.channel_id)
        .bind(item.amount_fen)
        .bind(item.credit)
        .bind(item.bonus)
        .bind(item.subject)
        .bind(item.expire_at)
        .fetch_one(db)
        .await
}

pub async fn find_order_by_no(db: &PgPool, order_no: &str) -> sqlx::Result<Option<OrderRow>> {
    let sql = format!("SELECT {ORDER_COLUMNS} FROM pay_orders WHERE order_no = $1");

    sqlx::query_as::<_, OrderRow>(&sql)
        .bind(order_no)
        .fetch_optional(db)
        .await
}

pub async fn find_order_owned(
    db: &PgPool,
    order_no: &str,
    user_id: Uuid,
) -> sqlx::Result<Option<OrderRow>> {
    let sql =
        format!("SELECT {ORDER_COLUMNS} FROM pay_orders WHERE order_no = $1 AND user_id = $2");

    sqlx::query_as::<_, OrderRow>(&sql)
        .bind(order_no)
        .bind(user_id)
        .fetch_optional(db)
        .await
}

pub async fn mark_paid(
    db: &PgPool,
    order_no: &str,
    trade_no: &str,
    pay_type: &str,
) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "UPDATE pay_orders SET status = $2, trade_no = $3, pay_type = $4, paid_at = now() WHERE order_no = $1 AND status = $5",
    )
    .bind(order_no)
    .bind(ORDER_PAID)
    .bind(trade_no)
    .bind(pay_type)
    .bind(ORDER_PENDING)
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

pub async fn record_notify(
    db: &PgPool,
    order_no: Option<&str>,
    trade_no: Option<&str>,
    raw_body: &str,
    sign_valid: bool,
    verify_result: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO pay_notifies (order_no, trade_no, raw_body, sign_valid, verify_result) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(order_no)
    .bind(trade_no)
    .bind(raw_body)
    .bind(sign_valid)
    .bind(verify_result)
    .execute(db)
    .await?;

    Ok(())
}
