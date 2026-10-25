//! 广场审核的集成测试。

use sqlx::PgPool;
use uuid::Uuid;

use tududu::plaza::{self, PublishRequest};

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5432/tududu_test".to_owned());

    let pool = tududu::db::connect(&url, 5).await.expect("连接测试库失败");
    tududu::db::migrate(&pool).await.expect("迁移失败");
    pool
}

async fn new_user(pool: &PgPool) -> Uuid {
    let short_code = format!("P{}", &Uuid::new_v4().simple().to_string()[..9]);

    sqlx::query_scalar("INSERT INTO users (short_code, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&short_code)
        .bind("广场测试用户")
        .fetch_one(pool)
        .await
        .expect("建用户失败")
}

fn request(title: &str, text: &str) -> PublishRequest {
    PublishRequest {
        title: title.to_owned(),
        prompt_text: text.to_owned(),
        model: "gpt-image-1".to_owned(),
        category_id: None,
        category_name: None,
        cover_asset_id: None,
        source_session_id: None,
    }
}

#[tokio::test]
async fn clean_prompt_goes_live_directly() {
    let pool = pool().await;
    let user = new_user(&pool).await;

    let row = plaza::publish(&pool, user, request("海边日落", "把背景换成海边日落"))
        .await
        .expect("发布失败");

    assert_eq!(row.status, plaza::store::STATUS_PUBLISHED);
}

#[tokio::test]
async fn word_hit_sends_prompt_to_review() {
    let pool = pool().await;
    let user = new_user(&pool).await;

    let word = format!("测试敏感词{}", &Uuid::new_v4().simple().to_string()[..6]);
    plaza::store::add_sensitive_word(&pool, &word)
        .await
        .expect("加词失败");

    let text = format!("描述里带了{word}这几个字");
    let row = plaza::publish(&pool, user, request("测试", &text))
        .await
        .expect("发布失败");

    assert_eq!(row.status, plaza::store::STATUS_REVIEWING);

    // 待审的作品不该出现在公开列表里
    let listed = plaza::store::list_published(&pool, None, None, None, 100)
        .await
        .expect("查列表失败");
    assert!(!listed.iter().any(|item| item.id == row.id));
}

#[tokio::test]
async fn search_matches_title_and_prompt() {
    let pool = pool().await;
    let user = new_user(&pool).await;

    let word = format!("检索词{}", &Uuid::new_v4().simple().to_string()[..6]);

    plaza::publish(&pool, user, request(&format!("标题含{word}"), "普通描述"))
        .await
        .expect("发布失败");

    let keyword = format!("%{word}%");

    let by_title = plaza::store::list_published(&pool, None, Some(&keyword), None, 50)
        .await
        .expect("搜索失败");
    assert_eq!(by_title.len(), 1);

    let missing = plaza::store::list_published(&pool, None, Some("%没有这个词%"), None, 50)
        .await
        .expect("搜索失败");
    assert!(missing.is_empty());
}

#[tokio::test]
async fn custom_category_is_created_once() {
    let pool = pool().await;
    let name = format!("自定义{}", &Uuid::new_v4().simple().to_string()[..6]);

    let first = plaza::store::ensure_category(&pool, &name)
        .await
        .expect("建分类失败");
    let second = plaza::store::ensure_category(&pool, &name)
        .await
        .expect("重复建分类失败");

    assert_eq!(first, second);
}
