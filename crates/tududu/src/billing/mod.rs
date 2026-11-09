//! 计费：钱包余额、流水、价格表。
//!
//! 余额单位由 billing_config 定义，可以按需要改成点、米粒或次数；
//! 金额是四位小数的定点数，够表达 0.0001 这样的单价。
pub mod store;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
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
    /// 单价，允许小数，最多四位。
    #[serde(deserialize_with = "deserialize_price")]
    pub price: Decimal,
}

/// 价格允许前端传数字或字符串，统一转成四位小数的定点数。
fn deserialize_price<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(f64),
        Text(String),
    }

    let value = Raw::deserialize(deserializer)?;
    let parsed = match value {
        Raw::Number(number) => {
            if !number.is_finite() {
                return Err(serde::de::Error::custom("价格不合法"));
            }
            crate::money::from_f64(number)
        }
        Raw::Text(text) => crate::money::parse_decimal(&text)
            .ok_or_else(|| serde::de::Error::custom("价格不合法"))?,
    };

    if parsed.is_sign_negative() {
        return Err(serde::de::Error::custom("价格必须是非负数"));
    }

    Ok(parsed)
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
    pub amount: Decimal,
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
) -> Result<Decimal, ApiError> {
    let found = store::find_price(db, model, size, quality).await?;

    found.ok_or_else(|| ApiError::bad_request("该模型还没有配置价格，请联系管理员"))
}

pub async fn charge(
    db: &PgPool,
    user_id: Uuid,
    amount: impl Into<Decimal>,
    ref_type: Option<&str>,
    ref_id: Option<Uuid>,
    remark: &str,
) -> Result<Decimal, ApiError> {
    let amount = amount.into();
    if amount <= Decimal::ZERO {
        return Ok(wallet(db, user_id).await?.balance);
    }

    apply_delta(db, user_id, -amount, KIND_CHARGE, ref_type, ref_id, remark).await
}

pub async fn refund(
    db: &PgPool,
    user_id: Uuid,
    amount: impl Into<Decimal>,
    ref_type: Option<&str>,
    ref_id: Option<Uuid>,
    remark: &str,
) -> Result<Decimal, ApiError> {
    let amount = amount.into();
    if amount <= Decimal::ZERO {
        return Ok(wallet(db, user_id).await?.balance);
    }

    apply_delta(db, user_id, amount, KIND_REFUND, ref_type, ref_id, remark).await
}

pub async fn grant(
    db: &PgPool,
    user_id: Uuid,
    amount: impl Into<Decimal>,
    remark: &str,
) -> Result<Decimal, ApiError> {
    let amount = amount.into();
    apply_delta(db, user_id, amount, KIND_GRANT, None, None, remark).await
}

/// 带业务引用的发放，用于保证同一笔订单重复回调只入账一次。
pub async fn grant_ref(
    db: &PgPool,
    user_id: Uuid,
    amount: impl Into<Decimal>,
    ref_type: &str,
    ref_id: Uuid,
    remark: &str,
) -> Result<Decimal, ApiError> {
    apply_delta(
        db,
        user_id,
        amount.into(),
        KIND_GRANT,
        Some(ref_type),
        Some(ref_id),
        remark,
    )
    .await
}

/// 改余额并记一条流水。整个过程在一个事务里，余额加行锁。
async fn apply_delta(
    db: &PgPool,
    user_id: Uuid,
    delta: Decimal,
    kind: &str,
    ref_type: Option<&str>,
    ref_id: Option<Uuid>,
    remark: &str,
) -> Result<Decimal, ApiError> {
    // 统一保留四位小数，避免累加出更多位
    let delta = delta.round_dp(4);
    let mut tx = db.begin().await?;

    sqlx::query("INSERT INTO wallets (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    let balance: Decimal =
        sqlx::query_scalar("SELECT balance FROM wallets WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    let after = (balance + delta).round_dp(4);
    if after.is_sign_negative() {
        return Err(ApiError::new(
            StatusCode::PAYMENT_REQUIRED,
            40201,
            "余额不足",
        ));
    }

    let credited = if kind == KIND_GRANT && delta > Decimal::ZERO {
        delta
    } else {
        Decimal::ZERO
    };

    // 先写流水。同一业务对象同一类型只会成功一次，重复调用直接返回原余额。
    let inserted: Option<Decimal> = sqlx::query_scalar(
        "INSERT INTO ledger_entries (user_id, kind, amount, balance_after, ref_type, ref_id, remark) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT DO NOTHING \
         RETURNING balance_after",
    )
    .bind(user_id)
    .bind(kind)
    .bind(delta)
    .bind(after)
    .bind(ref_type)
    .bind(ref_id)
    .bind(remark)
    .fetch_optional(&mut *tx)
    .await?;

    if inserted.is_none() {
        tx.rollback().await?;
        return Ok(balance);
    }

    sqlx::query(
        "UPDATE wallets SET balance = $2, total_charged = total_charged + $3, updated_at = now() WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(after)
    .bind(credited)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(after)
}

pub async fn update_config(
    db: &PgPool,
    req: UpdateConfigRequest,
) -> Result<store::ConfigRow, ApiError> {
    if req.unit_name.trim().is_empty() {
        return Err(ApiError::bad_request("单位名称不能为空"));
    }

    if req.unit_per_yuan <= 0 {
        return Err(ApiError::bad_request("每元单位数量需要大于 0"));
    }

    store::write_config(db, req.unit_name.trim(), req.unit_per_yuan).await?;

    config(db).await
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

        if item.price.is_sign_negative() {
            return Err(ApiError::bad_request("价格不能是负数"));
        }

        store::upsert_price(
            db,
            model,
            normalize(item.size.as_deref()).as_deref(),
            normalize(item.quality.as_deref()).as_deref(),
            item.price.round_dp(4),
        )
        .await?;
    }

    Ok(store::list_prices(db).await?)
}

/// 空白字符串当通配处理。
pub fn normalize(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}
