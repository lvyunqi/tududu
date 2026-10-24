//! 计费与支付的集成测试，需要真实 PostgreSQL。
//!
//! 每个用例自己建用户，互不干扰；数据库地址来自 TEST_DATABASE_URL。

use std::collections::BTreeMap;

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use tududu::billing;
use tududu::payment;
use tududu::provider::crypto;

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/tududu_test".to_owned());

    let pool = tududu::db::connect(&url, 5).await.expect("连接测试库失败");
    tududu::db::migrate(&pool).await.expect("迁移失败");
    pool
}

async fn new_user(pool: &PgPool) -> Uuid {
    let short_code = format!("T{}", &Uuid::new_v4().simple().to_string()[..9]);

    let id: Uuid =
        sqlx::query_scalar("INSERT INTO users (short_code, nickname) VALUES ($1, $2) RETURNING id")
            .bind(&short_code)
            .bind("测试用户")
            .fetch_one(pool)
            .await
            .expect("建用户失败");

    billing::store::ensure_wallet(pool, id)
        .await
        .expect("建钱包失败");
    id
}

async fn balance(pool: &PgPool, user: Uuid) -> i64 {
    billing::wallet(pool, user)
        .await
        .expect("读钱包失败")
        .balance
}

#[tokio::test]
async fn charge_and_refund_are_idempotent() {
    let pool = pool().await;
    let user = new_user(&pool).await;
    let order = Uuid::new_v4();

    billing::grant(&pool, user, 100, "测试发放")
        .await
        .expect("发放失败");
    assert_eq!(balance(&pool, user).await, 100);

    billing::charge(&pool, user, 30, Some("job"), Some(order), "扣费")
        .await
        .expect("扣费失败");
    assert_eq!(balance(&pool, user).await, 70);

    // 同一笔业务重复扣，不该再扣一次
    billing::charge(&pool, user, 30, Some("job"), Some(order), "扣费")
        .await
        .expect("重复扣费失败");
    assert_eq!(balance(&pool, user).await, 70);

    billing::refund(&pool, user, 30, Some("job"), Some(order), "退回")
        .await
        .expect("退款失败");
    assert_eq!(balance(&pool, user).await, 100);

    billing::refund(&pool, user, 30, Some("job"), Some(order), "退回")
        .await
        .expect("重复退款失败");
    assert_eq!(balance(&pool, user).await, 100);
}

#[tokio::test]
async fn charge_rejects_insufficient_balance() {
    let pool = pool().await;
    let user = new_user(&pool).await;

    billing::grant(&pool, user, 10, "测试发放")
        .await
        .expect("发放失败");

    let err = billing::charge(&pool, user, 20, None, None, "扣费")
        .await
        .expect_err("余额不足却没报错");

    assert_eq!(err.code, 40201);
    assert_eq!(balance(&pool, user).await, 10);
}

#[tokio::test]
async fn price_lookup_prefers_specific_rule() {
    let pool = pool().await;
    let model = format!("test-model-{}", &Uuid::new_v4().simple().to_string()[..6]);

    billing::store::upsert_price(&pool, &model, None, None, 10)
        .await
        .expect("写通配价失败");
    billing::store::upsert_price(&pool, &model, Some("1024x1024"), None, 20)
        .await
        .expect("写尺寸价失败");
    billing::store::upsert_price(&pool, &model, Some("1024x1024"), Some("high"), 30)
        .await
        .expect("写精确价失败");

    let wildcard = billing::price_for(&pool, &model, Some("512x512"), None)
        .await
        .expect("查通配价失败");
    assert_eq!(wildcard, 10);

    let sized = billing::price_for(&pool, &model, Some("1024x1024"), Some("low"))
        .await
        .expect("查尺寸价失败");
    assert_eq!(sized, 20);

    let exact = billing::price_for(&pool, &model, Some("1024x1024"), Some("high"))
        .await
        .expect("查精确价失败");
    assert_eq!(exact, 30);
}

struct PayFixture {
    user: Uuid,
    order_no: String,
    amount_fen: i64,
    credit: i64,
    secret: String,
    pid: String,
}

async fn pay_fixture(pool: &PgPool) -> PayFixture {
    let user = new_user(pool).await;
    let pid = format!("p{}", &Uuid::new_v4().simple().to_string()[..8]);
    let secret = "test-merchant-secret".to_owned();
    let key = crypto::derive_key("integration-test-key-material-32b");
    let cipher = crypto::encrypt(&key, &secret).expect("加密失败");

    let channel = payment::store::insert_channel(
        pool,
        "测试通道",
        "https://pay.example.com",
        &pid,
        &cipher,
        "alipay",
    )
    .await
    .expect("建通道失败");

    let plan = payment::store::insert_plan(pool, "测试套餐", 1200, 100, 10, 0)
        .await
        .expect("建套餐失败");

    let order_no = format!("T{}", &Uuid::new_v4().simple().to_string()[..18]);
    let created = payment::store::NewOrder {
        order_no: &order_no,
        user_id: user,
        plan_id: plan.id,
        channel_id: channel.id,
        amount_fen: plan.amount_fen,
        credit: plan.credit,
        bonus: plan.bonus,
        subject: "测试套餐",
        expire_at: Utc::now() + Duration::minutes(30),
    };

    payment::store::insert_order(pool, &created)
        .await
        .expect("建订单失败");

    PayFixture {
        user,
        order_no,
        amount_fen: plan.amount_fen,
        credit: plan.credit + plan.bonus,
        secret,
        pid,
    }
}

fn signed_params(fixture: &PayFixture, sign_with: &str) -> BTreeMap<String, String> {
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    params.insert("pid".to_owned(), fixture.pid.clone());
    params.insert("out_trade_no".to_owned(), fixture.order_no.clone());
    params.insert("trade_no".to_owned(), "GATEWAY123".to_owned());
    params.insert("type".to_owned(), "alipay".to_owned());
    params.insert("trade_status".to_owned(), "TRADE_SUCCESS".to_owned());
    params.insert("money".to_owned(), payment::fen_to_yuan(fixture.amount_fen));

    let signature = payment::sign(&params, sign_with);
    params.insert("sign".to_owned(), signature);
    params
}

#[tokio::test]
async fn epay_notify_credits_only_once() {
    let pool = pool().await;
    let fixture = pay_fixture(&pool).await;
    let key = crypto::derive_key("integration-test-key-material-32b");

    let params = signed_params(&fixture, &fixture.secret);

    let raw = params
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    let first = payment::handle_notify(&pool, &key, &raw, params.clone()).await;
    assert_eq!(first, "success");
    assert_eq!(balance(&pool, fixture.user).await, fixture.credit);

    let second = payment::handle_notify(&pool, &key, &raw, params).await;
    assert_eq!(second, "success");
    assert_eq!(balance(&pool, fixture.user).await, fixture.credit);
}

#[tokio::test]
async fn epay_notify_rejects_bad_signature() {
    let pool = pool().await;
    let fixture = pay_fixture(&pool).await;
    let key = crypto::derive_key("integration-test-key-material-32b");

    let params = signed_params(&fixture, "wrong-secret");

    let result = payment::handle_notify(&pool, &key, "raw=body", params).await;
    assert_eq!(result, "fail");
    assert_eq!(balance(&pool, fixture.user).await, 0);
}
