use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WalletRow {
    pub user_id: Uuid,
    #[serde(serialize_with = "crate::money::serialize_decimal")]
    pub balance: Decimal,
    #[serde(serialize_with = "crate::money::serialize_decimal")]
    pub total_charged: Decimal,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LedgerRow {
    pub id: i64,
    pub kind: String,
    #[serde(serialize_with = "crate::money::serialize_decimal")]
    pub amount: Decimal,
    #[serde(serialize_with = "crate::money::serialize_decimal")]
    pub balance_after: Decimal,
    pub ref_type: Option<String>,
    pub ref_id: Option<Uuid>,
    pub remark: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PriceRow {
    pub id: Uuid,
    pub model: String,
    pub size: Option<String>,
    pub quality: Option<String>,
    #[serde(serialize_with = "crate::money::serialize_decimal")]
    pub price: Decimal,
    pub enabled: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConfigRow {
    pub unit_name: String,
    pub unit_per_yuan: i64,
    /// 快捷充值金额，逗号分隔，单位分
    pub quick_amounts: String,
}

impl ConfigRow {
    /// 快捷金额解析成整数列表，坏数据直接跳过。
    pub fn quick_amount_fen(&self) -> Vec<i64> {
        self.quick_amounts
            .split(',')
            .filter_map(|item| item.trim().parse::<i64>().ok())
            .filter(|fen| *fen > 0)
            .collect()
    }
}

pub async fn find_wallet(db: &PgPool, user_id: Uuid) -> sqlx::Result<Option<WalletRow>> {
    sqlx::query_as::<_, WalletRow>(
        "SELECT user_id, balance, total_charged, updated_at FROM wallets WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn ensure_wallet(db: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO wallets (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn list_ledger(
    db: &PgPool,
    user_id: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> sqlx::Result<Vec<LedgerRow>> {
    sqlx::query_as::<_, LedgerRow>(
        "SELECT id, kind, amount, balance_after, ref_type, ref_id, remark, created_at \
           FROM ledger_entries \
          WHERE user_id = $1 AND ($2::timestamptz IS NULL OR created_at < $2) \
          ORDER BY id DESC LIMIT $3",
    )
    .bind(user_id)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await
}

pub async fn read_config(db: &PgPool) -> sqlx::Result<Option<ConfigRow>> {
    sqlx::query_as::<_, ConfigRow>(
        "SELECT unit_name, unit_per_yuan, quick_amounts FROM billing_config WHERE id = 1",
    )
    .fetch_optional(db)
    .await
}

pub async fn write_config(
    db: &PgPool,
    unit_name: &str,
    unit_per_yuan: i64,
    quick_amounts: Option<&[i64]>,
) -> sqlx::Result<()> {
    match quick_amounts {
        Some(list) => {
            let joined = list
                .iter()
                .map(|fen| fen.to_string())
                .collect::<Vec<_>>()
                .join(",");

            sqlx::query("UPDATE billing_config SET unit_name = $1, unit_per_yuan = $2, quick_amounts = $3, updated_at = now() WHERE id = 1")
                .bind(unit_name)
                .bind(unit_per_yuan)
                .bind(joined)
                .execute(db)
                .await?;
        }
        None => {
            sqlx::query("UPDATE billing_config SET unit_name = $1, unit_per_yuan = $2, updated_at = now() WHERE id = 1")
                .bind(unit_name)
                .bind(unit_per_yuan)
                .execute(db)
                .await?;
        }
    }

    Ok(())
}

pub async fn list_prices(db: &PgPool) -> sqlx::Result<Vec<PriceRow>> {
    sqlx::query_as::<_, PriceRow>(
        "SELECT id, model, size, quality, price, enabled FROM billing_prices ORDER BY model, size NULLS FIRST, quality NULLS FIRST",
    )
    .fetch_all(db)
    .await
}

pub async fn upsert_price(
    db: &PgPool,
    model: &str,
    size: Option<&str>,
    quality: Option<&str>,
    price: impl Into<Decimal>,
) -> sqlx::Result<PriceRow> {
    let price: Decimal = price.into();
    sqlx::query_as::<_, PriceRow>(
        "INSERT INTO billing_prices (model, size, quality, price) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (model, size, quality) DO UPDATE SET price = EXCLUDED.price, enabled = TRUE \
         RETURNING id, model, size, quality, price, enabled",
    )
    .bind(model)
    .bind(size)
    .bind(quality)
    .bind(price)
    .fetch_one(db)
    .await
}

/// 取匹配度最高的价格：先精确，再按尺寸或质量通配，最后整模型通配。
pub async fn find_price(
    db: &PgPool,
    model: &str,
    size: Option<&str>,
    quality: Option<&str>,
) -> sqlx::Result<Option<Decimal>> {
    sqlx::query_scalar::<_, Decimal>(
        "SELECT price FROM billing_prices \
          WHERE model = $1 AND enabled = TRUE \
            AND (size IS NULL OR size = $2) \
            AND (quality IS NULL OR quality = $3) \
          ORDER BY (size IS NOT NULL)::int + (quality IS NOT NULL)::int DESC \
          LIMIT 1",
    )
    .bind(model)
    .bind(size)
    .bind(quality)
    .fetch_optional(db)
    .await
}
