//! 在线配置的集成测试。用例串行执行，靠 CI 的 --test-threads=1。

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5432/tududu_test".to_owned());

    let pool = tududu::db::connect(&url, 5).await.expect("连接测试库失败");
    tududu::db::migrate(&pool).await.expect("迁移失败");
    pool
}

fn sample() -> tududu::settings::LiveSettings {
    tududu::settings::LiveSettings {
        access_ttl_secs: 900,
        refresh_ttl_secs: 2_592_000,
        user_quota_bytes: 2_147_483_648,
        trash_retention_days: 30,
        public_base_url: None,
    }
}

#[tokio::test]
async fn saved_value_is_loaded_back() {
    let pool = pool().await;

    let next = tududu::settings::save(&pool, "trash_retention_days", "15", &sample())
        .await
        .expect("保存失败");
    assert_eq!(next.trash_retention_days, 15);

    let stored = tududu::settings::load_stored(&pool).await.expect("读取失败");
    assert_eq!(stored.get("trash_retention_days").map(String::as_str), Some("15"));
}

#[tokio::test]
async fn invalid_value_is_rejected_before_writing() {
    let pool = pool().await;

    let err = tududu::settings::save(&pool, "trash_retention_days", "0", &sample())
        .await
        .expect_err("越界值却通过了");
    assert_eq!(err.code, 10001);

    let stored = tududu::settings::load_stored(&pool).await.expect("读取失败");
    assert_ne!(
        stored.get("trash_retention_days").map(String::as_str),
        Some("0")
    );
}

#[tokio::test]
async fn stored_value_overrides_config() {
    let pool = pool().await;

    tududu::settings::save(&pool, "user_quota_bytes", "1024", &sample())
        .await
        .expect("保存失败");

    let stored = tududu::settings::load_stored(&pool).await.expect("读取失败");
    let mut live = sample();

    for (key, raw) in &stored {
        if let Ok(next) = live.with(key, raw) {
            live = next;
        }
    }

    assert_eq!(live.user_quota_bytes, 1024);
}
