//! 会话相关的集成测试，需要真实 PostgreSQL。

use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5432/tududu_test".to_owned());

    let pool = tududu::db::connect(&url, 5).await.expect("连接测试库失败");
    tududu::db::migrate(&pool).await.expect("迁移失败");
    pool
}

async fn new_user(pool: &PgPool) -> Uuid {
    let short_code = format!("S{}", &Uuid::new_v4().simple().to_string()[..9]);

    sqlx::query_scalar("INSERT INTO users (short_code, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&short_code)
        .bind("会话测试用户")
        .fetch_one(pool)
        .await
        .expect("建用户失败")
}

async fn new_session(pool: &PgPool, user: Uuid) -> Uuid {
    sqlx::query_scalar("INSERT INTO sessions (user_id, model) VALUES ($1, $2) RETURNING id")
        .bind(user)
        .bind("gpt-image-1")
        .fetch_one(pool)
        .await
        .expect("建会话失败")
}

async fn new_asset(pool: &PgPool, owner: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO assets (owner_user_id, object_key, mime, byte_size, sha256) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(owner)
    .bind(format!("g/{owner}/test.png"))
    .bind("image/png")
    .bind(1024_i64)
    .bind(format!("{}", Uuid::new_v4().simple()))
    .fetch_one(pool)
    .await
    .expect("建资产失败")
}

async fn add_assistant_output(pool: &PgPool, session: Uuid, asset: Uuid) {
    sqlx::query(
        "INSERT INTO session_messages (session_id, role, content, output_asset_id, status) \
         VALUES ($1, 'assistant', '', $2, 3)",
    )
    .bind(session)
    .bind(asset)
    .execute(pool)
    .await
    .expect("写助手消息失败");
}

#[tokio::test]
async fn latest_output_asset_follows_the_newest_result() {
    let pool = pool().await;
    let user = new_user(&pool).await;
    let session = new_session(&pool, user).await;

    let first = new_asset(&pool, user).await;
    add_assistant_output(&pool, session, first).await;

    let found = tududu::session::store::latest_output_asset(&pool, session)
        .await
        .expect("查询失败");
    assert_eq!(found, Some(first));

    let second = new_asset(&pool, user).await;
    add_assistant_output(&pool, session, second).await;

    let found = tududu::session::store::latest_output_asset(&pool, session)
        .await
        .expect("查询失败");
    assert_eq!(found, Some(second));
}

#[tokio::test]
async fn empty_session_has_no_previous_output() {
    let pool = pool().await;
    let user = new_user(&pool).await;
    let session = new_session(&pool, user).await;

    let found = tududu::session::store::latest_output_asset(&pool, session)
        .await
        .expect("查询失败");
    assert_eq!(found, None);
}
