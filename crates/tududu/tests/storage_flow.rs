//! 存储与回收站的集成测试。

use sqlx::PgPool;
use uuid::Uuid;

use tududu::storage::local_store::LocalStore;

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5432/tududu_test".to_owned());

    let pool = tududu::db::connect(&url, 5).await.expect("连接测试库失败");
    tududu::db::migrate(&pool).await.expect("迁移失败");
    pool
}

async fn new_user(pool: &PgPool) -> Uuid {
    let short_code = format!("R{}", &Uuid::new_v4().simple().to_string()[..9]);

    sqlx::query_scalar("INSERT INTO users (short_code, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&short_code)
        .bind("存储测试用户")
        .fetch_one(pool)
        .await
        .expect("建用户失败")
}

async fn new_asset(pool: &PgPool, owner: Uuid, status: i16, days_ago: Option<i64>) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO assets (owner_user_id, object_key, thumb_key, mime, byte_size, sha256, status) \
         VALUES ($1, $2, $3, 'image/png', 1024, $4, $5) RETURNING id",
    )
    .bind(owner)
    .bind(format!("g/{owner}/{}/x.png", Uuid::new_v4().simple()))
    .bind(format!("g/{owner}/{}/x_thumb.jpg", Uuid::new_v4().simple()))
    .bind(format!("{}", Uuid::new_v4().simple()))
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("建资产失败");

    if let Some(days) = days_ago {
        sqlx::query(
            "UPDATE assets SET trashed_at = now() - make_interval(days => $2) WHERE id = $1",
        )
        .bind(id)
        .bind(days as i32)
        .execute(pool)
        .await
        .expect("设置回收时间失败");
    }

    id
}

async fn status_of(pool: &PgPool, id: Uuid) -> i16 {
    sqlx::query_scalar("SELECT status FROM assets WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("读状态失败")
}

fn store() -> LocalStore {
    LocalStore::new(std::env::temp_dir().join("tududu-test-data"))
}

#[tokio::test]
async fn expired_trash_is_purged() {
    let pool = pool().await;
    let user = new_user(&pool).await;

    let old = new_asset(&pool, user, 2, Some(40)).await;
    let fresh = new_asset(&pool, user, 2, Some(1)).await;
    let alive = new_asset(&pool, user, 1, None).await;

    let purged = tududu::tasks::purge_trash(&pool, &store(), 30)
        .await
        .expect("清理失败");

    assert!(purged >= 1);
    assert_eq!(status_of(&pool, old).await, 3);
    assert_eq!(status_of(&pool, fresh).await, 2);
    assert_eq!(status_of(&pool, alive).await, 1);
}

#[tokio::test]
async fn soft_delete_stamps_trash_time() {
    let pool = pool().await;
    let user = new_user(&pool).await;
    let id = new_asset(&pool, user, 1, None).await;

    let removed = tududu::storage::store::soft_delete(&pool, id, user)
        .await
        .expect("软删失败");
    assert!(removed);

    let trashed_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT trashed_at FROM assets WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("读回收时间失败");

    assert!(trashed_at.is_some());
}
