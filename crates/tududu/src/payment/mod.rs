//! 易支付接入：签名、下单、回调。

pub mod store;

use std::collections::BTreeMap;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use md5::{Digest, Md5};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::provider::crypto;

const ORDER_TTL_MINUTES: i64 = 30;

#[derive(Debug, Serialize)]
pub struct CheckoutInfo {
    pub order_no: String,
    pub amount_fen: i64,
    pub credit: i64,
    pub bonus: i64,
    pub subject: String,
    pub pay_url: String,
    pub expire_at: chrono::DateTime<Utc>,
}

/// 易支付签名：去掉 sign 与 sign_type，按参数名升序拼 k=v，末尾接商户密钥，取 MD5 小写。
pub fn sign(params: &BTreeMap<String, String>, key: &str) -> String {
    let mut pairs: Vec<String> = Vec::new();

    for (name, value) in params {
        if name == "sign" || name == "sign_type" || value.is_empty() {
            continue;
        }

        pairs.push(format!("{name}={value}"));
    }

    let joined = pairs.join("&");
    let digest = Md5::digest(format!("{joined}{key}").as_bytes());
    hex::encode(digest)
}

pub fn verify(params: &BTreeMap<String, String>, key: &str, provided: &str) -> bool {
    let expected = sign(params, key);
    let actual = provided.to_ascii_lowercase();
    constant_time_eq(expected.as_bytes(), actual.as_bytes())
}

/// 分转成易支付要求的两位小数字符串。
pub fn fen_to_yuan(fen: i64) -> String {
    format!("{}.{:02}", fen / 100, fen % 100)
}

fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

pub async fn list_plans(db: &PgPool) -> Result<Vec<store::PlanRow>, ApiError> {
    Ok(store::list_plans(db).await?)
}

pub async fn create_order(
    db: &PgPool,
    encrypt_key: &[u8; 32],
    public_base: Option<&str>,
    user_id: Uuid,
    plan_id: Uuid,
) -> Result<CheckoutInfo, ApiError> {
    let Some(base) = public_base else {
        return Err(ApiError::internal("未配置对外访问地址，无法生成回调地址"));
    };

    let plan = store::find_plan(db, plan_id)
        .await?
        .ok_or_else(|| ApiError::not_found("套餐不存在"))?;

    if !plan.enabled {
        return Err(ApiError::bad_request("该套餐已下架"));
    }

    let found = store::pick_channel(db).await?;
    let channel = match found {
        Some(value) => value,
        None => {
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                40204,
                "暂时没有可用的支付通道",
            ));
        }
    };

    let secret = crypto::decrypt(encrypt_key, &channel.key_cipher).map_err(|err| {
        tracing::error!(error = %err, "解密支付密钥失败");
        ApiError::internal("支付通道配置异常")
    })?;

    let order_no = new_order_no();
    let subject = sanitize_subject(&plan.name);
    let expire_at = Utc::now() + Duration::minutes(ORDER_TTL_MINUTES);

    let created = store::NewOrder {
        order_no: &order_no,
        user_id,
        plan_id: plan.id,
        channel_id: channel.id,
        amount_fen: plan.amount_fen,
        credit: plan.credit,
        bonus: plan.bonus,
        subject: &subject,
        expire_at,
    };

    store::insert_order(db, &created).await?;

    let base = base.trim_end_matches('/');
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    params.insert("pid".to_owned(), channel.pid.clone());
    params.insert("type".to_owned(), channel.pay_type.clone());
    params.insert("out_trade_no".to_owned(), order_no.clone());
    params.insert("notify_url".to_owned(), format!("{base}/pay/epay/notify"));
    params.insert("return_url".to_owned(), format!("{base}/pay/epay/return"));
    params.insert("name".to_owned(), subject.clone());
    params.insert("money".to_owned(), fen_to_yuan(plan.amount_fen));

    let signature = sign(&params, &secret);
    params.insert("sign".to_owned(), signature);
    params.insert("sign_type".to_owned(), "MD5".to_owned());

    let query = params
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .collect::<Vec<_>>()
        .join("&");

    let gateway = channel.gateway_url.trim_end_matches('/');

    Ok(CheckoutInfo {
        order_no,
        amount_fen: plan.amount_fen,
        credit: plan.credit,
        bonus: plan.bonus,
        subject,
        pay_url: format!("{gateway}/submit.php?{query}"),
        expire_at,
    })
}

pub async fn order_status(
    db: &PgPool,
    user_id: Uuid,
    order_no: &str,
) -> Result<store::OrderRow, ApiError> {
    store::find_order_owned(db, order_no, user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))
}

/// 处理异步通知。返回的字符串直接作为响应体，易支付只认 success。
pub async fn handle_notify(
    db: &PgPool,
    encrypt_key: &[u8; 32],
    raw_body: &str,
    params: BTreeMap<String, String>,
) -> String {
    let Some(pid) = params.get("pid").cloned() else {
        audit(db, &params, raw_body, false, "missing_pid").await;
        return "fail".to_owned();
    };

    let channel = match store::find_channel_by_pid(db, &pid).await {
        Ok(Some(found)) => found,
        Ok(None) => {
            audit(db, &params, raw_body, false, "unknown_pid").await;
            return "fail".to_owned();
        }
        Err(err) => {
            tracing::error!(error = %err, "查询支付通道失败");
            return "fail".to_owned();
        }
    };

    let secret = match crypto::decrypt(encrypt_key, &channel.key_cipher) {
        Ok(value) => value,
        Err(err) => {
            tracing::error!(error = %err, "解密支付密钥失败");
            audit(db, &params, raw_body, false, "decrypt_failed").await;
            return "fail".to_owned();
        }
    };

    let provided = params.get("sign").cloned().unwrap_or_default();
    if !verify(&params, &secret, &provided) {
        audit(db, &params, raw_body, false, "sign_mismatch").await;
        return "fail".to_owned();
    }

    let Some(order_no) = params.get("out_trade_no").cloned() else {
        audit(db, &params, raw_body, true, "missing_order_no").await;
        return "fail".to_owned();
    };

    let order = match store::find_order_by_no(db, &order_no).await {
        Ok(Some(found)) => found,
        Ok(None) => {
            audit(db, &params, raw_body, true, "order_not_found").await;
            return "fail".to_owned();
        }
        Err(err) => {
            tracing::error!(error = %err, "查询订单失败");
            return "fail".to_owned();
        }
    };

    if order.status == store::ORDER_PAID {
        audit(db, &params, raw_body, true, "already_paid").await;
        return "success".to_owned();
    }

    let status_text = params.get("trade_status").cloned().unwrap_or_default();
    if status_text != "TRADE_SUCCESS" {
        audit(db, &params, raw_body, true, "not_success").await;
        return "success".to_owned();
    }

    let paid_fen = params
        .get("money")
        .and_then(|value| parse_yuan(value))
        .unwrap_or_default();

    if paid_fen != order.amount_fen {
        audit(db, &params, raw_body, true, "amount_mismatch").await;
        return "fail".to_owned();
    }

    let total = order.credit + order.bonus;
    let charged =
        crate::billing::grant_ref(db, order.user_id, total, "order", order.id, "充值").await;

    if let Err(err) = charged {
        tracing::error!(error = %err, %order_no, "充值入账失败");
        return "fail".to_owned();
    }

    let trade_no = params.get("trade_no").cloned().unwrap_or_default();
    let pay_type = params.get("type").cloned().unwrap_or_default();

    if let Err(err) = store::mark_paid(db, &order_no, &trade_no, &pay_type).await {
        tracing::error!(error = %err, %order_no, "更新订单状态失败");
        return "fail".to_owned();
    }

    audit(db, &params, raw_body, true, "ok").await;
    "success".to_owned()
}

/// 记录回调原文与处理结果，出错只记日志，不影响返回。
async fn audit(
    db: &PgPool,
    params: &BTreeMap<String, String>,
    raw_body: &str,
    sign_valid: bool,
    verify_result: &str,
) {
    let order_no = params.get("out_trade_no").map(String::as_str);
    let trade_no = params.get("trade_no").map(String::as_str);

    let trimmed: String = raw_body.chars().take(4000).collect();
    let outcome =
        store::record_notify(db, order_no, trade_no, &trimmed, sign_valid, verify_result).await;

    if let Err(err) = outcome {
        tracing::error!(error = %err, "写回调记录失败");
    }
}

/// 解析易支付回传的金额，只接受最多两位小数。
pub fn parse_yuan(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split('.');
    let yuan = parts.next()?;
    let cents = parts.next().unwrap_or("0");

    if parts.next().is_some() || cents.len() > 2 {
        return None;
    }

    let yuan: i64 = yuan.parse().ok()?;
    let cents: i64 = match cents.len() {
        0 => 0,
        1 => cents.parse::<i64>().ok()? * 10,
        _ => cents.parse().ok()?,
    };

    if yuan < 0 || cents < 0 {
        return None;
    }

    Some(yuan * 100 + cents)
}

fn new_order_no() -> String {
    let stamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let raw = Uuid::new_v4().into_bytes();
    let suffix: String = raw
        .iter()
        .take(6)
        .map(|byte| char::from(b'0' + (byte % 10)))
        .collect();

    format!("{stamp}{suffix}")
}

/// 商品名里去掉易支付不接受的符号，并限制长度。
fn sanitize_subject(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| !matches!(c, '&' | '=' | '?' | '#' | '/' | '%' | '+' | '<' | '>'))
        .take(32)
        .collect();

    if cleaned.trim().is_empty() {
        "图嘟嘟充值".to_owned()
    } else {
        cleaned.trim().to_owned()
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b) {
        diff |= left ^ right;
    }

    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        params.insert("pid".to_owned(), "1001".to_owned());
        params.insert("out_trade_no".to_owned(), "20261025010101abcdef".to_owned());
        params.insert("money".to_owned(), "12.00".to_owned());
        params
    }

    #[test]
    fn sign_is_sorted_and_appends_key() {
        let params = sample();
        let text = "money=12.00&out_trade_no=20261025010101abcdef&pid=1001secret";

        assert_eq!(
            sign(&params, "secret"),
            hex::encode(Md5::digest(text.as_bytes()))
        );
    }

    #[test]
    fn sign_skips_sign_fields_and_empty_values() {
        let mut params = sample();
        params.insert("sign".to_owned(), "whatever".to_owned());
        params.insert("sign_type".to_owned(), "MD5".to_owned());
        params.insert("empty".to_owned(), String::new());

        assert_eq!(sign(&params, "secret"), sign(&sample(), "secret"));
    }

    #[test]
    fn verify_accepts_uppercase_signature() {
        let params = sample();
        let signature = sign(&params, "secret");

        assert!(verify(&params, "secret", &signature.to_uppercase()));
        assert!(!verify(&params, "secret", "deadbeef"));
    }

    #[test]
    fn yuan_formatting_round_trips() {
        assert_eq!(fen_to_yuan(1_200), "12.00");
        assert_eq!(fen_to_yuan(5), "0.05");
        assert_eq!(parse_yuan("12.00"), Some(1_200));
        assert_eq!(parse_yuan("12.5"), Some(1_250));
        assert_eq!(parse_yuan("12"), Some(1_200));
        assert_eq!(parse_yuan("12.345"), None);
        assert_eq!(parse_yuan(""), None);
    }

    #[test]
    fn subject_drops_reserved_characters() {
        assert_eq!(sanitize_subject("60 元=600&点"), "60 元600点");
        assert_eq!(sanitize_subject("   "), "图嘟嘟充值");
    }
}
