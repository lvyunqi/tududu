//! 计费：钱包余额、流水、价格表。
//!
//! 余额是不带小数的整数，单位由 billing_config 定义，可以按需要改成点、米粒或次数。

pub mod store;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

pub const KIND_CHARGE: &str = "charge";
pub const KIND_REFUND: &str = "refund";
pub const KIND_GRANT: &str = "grant";

#[derive(Debug, Deserialize)]
pub struct PriceItem {
    pub model: String,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    pub price: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricesRequest {
    pub items: Vec<PriceItem>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub unit_name: String,
    pub unit_per_yuan: i64,
}

#[derive(Debug, Deserialize)]
pub struct AdjustCreditsRequest {
    pub amount: i64,
    #[serde(default)]
    pub remark: Option<String>,
}

pub async fn wallet(db: &PgPool, user_id: Uuid) -> Result<store::WalletRow, ApiError> {
    store::ensure_wallet(db, user_id).await?;

    store::find_wallet(db, user_id)
        .await?
        .ok_or_else(|| ApiError::internal("钱包不存在"))
}

pub async fn ledger(
    db: &PgPool,
    user_id: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<store::LedgerRow>, ApiError> {
    Ok(store::list_ledger(db, user_id, before, limit).await?)
}

pub async fn config(db: &PgPool) -> Result<store::ConfigRow, ApiError> {
    store::read_config(db)
        .await?
        .ok_or_else(|| ApiError::internal("计费配置缺失"))
}

/// 查价格。没配价格就不允许提交任务，避免免费刷图。
pub async fn price_for(
    db: &PgPool,
    model: &str,
    size: Option<&str>,
    quality: Option<&str>,
) -> Result<i64, ApiError> {
    let found = store::find_price(db, model, size, quality).await?;

    found.ok_or_else(|| ApiError::bad_request("该模型还没有配置价格，请联系管理员"))
}

pub async fn charge(
    db: &PgPool,
    user_id: Uuid,
    amount: i64,
    ref_type: Option<&str>,
    ref_id: Option<Uuid>,
    remark: &str,
) -> Result<i64, ApiError> {
    if amount <= 0 {
        return Ok(wallet(db, user_id).await?.balance);
    }

    apply_delta(db, user_id, -amount, KIND_CHARGE, ref_type, ref_id, remark).await
}

pub async fn refund(
    db: &PgPool,
    user_id: Uuid,
    amount: i64,
    ref_type: Option<&str>,
    ref_id: Option<Uuid>,
    remark: &str,
) -> Result<i64, ApiError> {
    if amount <= 0 {
        return Ok(wallet(db, user_id).await?.balance);
    }

    apply_delta(db, user_id, amount, KIND_REFUND, ref_type, ref_id, remark).await
}

pub async fn grant(db: &PgPool, user_id: Uuid, amount: i64, remark: &str) -> Result<i64, ApiError> {
    apply_delta(db, user_id, amount, KIND_GRANT, None, None, remark).await
}

/// 改余额并记一条流水。整个过程在一个事务里，余额加行锁。
async fn apply_delta(
    db: &PgPool,
    user_id: Uuid,
    delta: i64,
    kind: &str,
    ref_type: Option<&str>,
    ref_id: Option<Uuid>,
    remark: &str,
) -> Result<i64, ApiError> {
    let mut tx = db.begin().await?;

    sqlx::query("INSERT INTO wallets (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let balance: i64 =
        sqlx::query_scalar("SELECT balance FROM wallets WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    let after = balance.saturating_add(delta);
    if after < 0 {
        return Err(ApiError::new(
            StatusCode::PAYMENT_REQUIRED,
            40201,
            "余额不足",
        ));
    }

    let credited = if kind == KIND_GRANT { delta.max(0) } else { 0 };

    sqlx::query(
        "UPDATE wallets SET balance = $2, total_charged = total_charged + $3, updated_at = now() WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(after)
    .bind(credited)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO ledger_entries (user_id, kind, amount, balance_after, ref_type, ref_id, remark) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(user_id)
    .bind(kind)
    .bind(delta)
    .bind(after)
    .bind(ref_type)
    .bind(ref_id)
    .bind(remark)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(after)
}

pub async fn update_prices(
    db: &PgPool,
    req: UpdatePricesRequest,
) -> Result<Vec<store::PriceRow>, ApiError> {
    for item in &req.items {
        let model = item.model.trim();
        if model.is_empty() {
            return Err(ApiError::bad_request("模型名不能为空"));
        }

        if item.price < 0 {
            return Err(ApiError::bad_request("价格不能是负数"));
        }

        store::upsert_price(
            db,
            model,
            normalize(item.size.as_deref()).as_deref(),
            normalize(item.quality.as_deref()).as_deref(),
            item.price,
        )
        .await?;
    }

    Ok(store::list_prices(db).await?)
}

pub async fn update_config(
    db: &PgPool,
    req: UpdateConfigRequest,
) -> Result<store::ConfigRow, ApiError> {
    let name = req.unit_name.trim();
    if name.is_empty() || name.chars().count() > 16 {
        return Err(ApiError::bad_request("单位名称需要 1 到 16 个字"));
    }

    if req.unit_per_yuan <= 0 {
        return Err(ApiError::bad_request("每元对应的单位数量必须大于 0"));
    }

    store::write_config(db, name, req.unit_per_yuan).await?;
    config(db).await
}

fn normalize(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_wildcards_become_none() {
        assert_eq!(normalize(Some("  ")), None);
        assert_eq!(normalize(None), None);
        assert_eq!(normalize(Some(" 1024x1024 ")), Some("1024x1024".to_owned()));
    }
}
