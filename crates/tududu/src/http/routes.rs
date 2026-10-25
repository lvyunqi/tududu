use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::extract::State;
use axum::routing::{delete, get, patch, post};
use serde::Serialize;
use sqlx::PgPool;

use crate::config::Config;
use crate::error::ApiBody;
use crate::http::{
    admin, admin_auth, admin_ops, assets, auth, billing, jobs, payment, plaza, sessions,
    static_files,
};

/// 进程级共享状态。连接池句柄可 Clone，配置只读所以放进 Arc。
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    /// 渠道密钥加解密用的密钥，启动时由主密钥派生。
    pub provider_key: [u8; 32],
    /// 访问上游用的 HTTP 客户端，全局复用一个连接池。
    pub http: reqwest::Client,
    /// 图片文件的本地存储。
    pub files: crate::storage::local_store::LocalStore,
    /// 进程内限流器，挡住密码爆破与刷接口。
    pub limiter: Arc<crate::ratelimit::RateLimiter>,
    /// 后台可改的运行时配置，改完立刻写回这里。
    pub settings: Arc<std::sync::RwLock<crate::settings::LiveSettings>>,
}

impl AppState {
    /// 取一份当前生效的配置。读锁只在克隆期间持有，不会跨越 await。
    pub fn live(&self) -> crate::settings::LiveSettings {
        self.settings
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 写入新的配置。调用方负责先落库，再调这里。
    pub fn set_live(&self, next: crate::settings::LiveSettings) {
        let mut guard = self
            .settings
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        *guard = next;
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/me", get(auth::me))
        .route(
            "/api/v1/uploads",
            post(assets::upload).layer(DefaultBodyLimit::max(
                crate::storage::MAX_UPLOAD_BYTES + 1024 * 1024,
            )),
        )
        .route("/api/v1/storage/quota", get(assets::quota))
        .route("/api/v1/assets", get(assets::list))
        .route(
            "/api/v1/assets/{id}",
            get(assets::detail).delete(assets::delete),
        )
        .route("/api/v1/assets/{id}/file", get(assets::file))
        .route("/api/v1/billing/wallet", get(billing::wallet))
        .route("/api/v1/billing/ledger", get(billing::ledger))
        .route("/api/v1/billing/config", get(billing::public_config))
        .route("/api/v1/billing/plans", get(payment::list_plans))
        .route("/api/v1/billing/orders", post(payment::create_order))
        .route(
            "/api/v1/billing/orders/{order_no}",
            get(payment::order_status),
        )
        .route(
            "/pay/epay/notify",
            get(payment::notify_get).post(payment::notify_post),
        )
        .route("/pay/epay/return", get(payment::return_page))
        .route("/api/v1/jobs", post(jobs::create))
        .route("/api/v1/jobs/{id}", get(jobs::detail))
        .route(
            "/api/v1/sessions",
            get(sessions::list).post(sessions::create),
        )
        .route(
            "/api/v1/sessions/{id}",
            get(sessions::detail)
                .patch(sessions::rename)
                .delete(sessions::delete),
        )
        .route(
            "/api/v1/sessions/{id}/messages",
            post(sessions::send_message),
        )
        .route("/api/v1/prompt-categories", get(plaza::categories))
        .route("/api/v1/prompts", get(plaza::list).post(plaza::publish))
        .route("/api/v1/my/prompts", get(plaza::mine))
        .route(
            "/api/v1/prompts/{code}",
            get(plaza::detail).delete(plaza::unpublish),
        )
        .route("/api/v1/prompts/{code}/like", post(plaza::like))
        .route(
            "/api/v1/admin/providers",
            get(admin::list_providers).post(admin::create_provider),
        )
        .route(
            "/api/v1/admin/providers/{id}",
            patch(admin::update_provider).delete(admin::delete_provider),
        )
        .route(
            "/api/v1/admin/providers/{id}/channels",
            get(admin::list_channels).post(admin::create_channel),
        )
        .route(
            "/api/v1/admin/providers/{id}/models",
            get(admin::list_models),
        )
        .route(
            "/api/v1/admin/providers/{id}/models/refresh",
            post(admin::refresh_models),
        )
        .route(
            "/api/v1/admin/billing/prices",
            get(billing::list_prices).put(billing::update_prices),
        )
        .route(
            "/api/v1/admin/billing/config",
            get(billing::admin_config).put(billing::update_config),
        )
        .route(
            "/api/v1/admin/users/{id}/credits",
            post(billing::adjust_credits),
        )
        .route(
            "/api/v1/admin/recharge-plans",
            get(payment::admin_list_plans).post(payment::admin_create_plan),
        )
        .route(
            "/api/v1/admin/pay-channels",
            get(payment::admin_list_channels).post(payment::admin_create_channel),
        )
        .route("/api/v1/admin/auth/login", post(admin_auth::login))
        .route("/api/v1/admin/overview", get(admin_ops::overview))
        .route("/api/v1/admin/reconcile", get(admin_ops::reconcile))
        .route("/api/v1/admin/settings", get(admin_ops::list_settings))
        .route(
            "/api/v1/admin/settings/{key}",
            axum::routing::put(admin_ops::update_setting),
        )
        .route("/api/v1/admin/users", get(admin_ops::list_users))
        .route(
            "/api/v1/admin/users/{id}/status",
            patch(admin_ops::set_user_status),
        )
        .route("/api/v1/admin/pay-orders", get(admin_ops::list_orders))
        .route("/api/v1/admin/jobs", get(admin_ops::list_jobs))
        .route("/api/v1/admin/jobs/{id}", get(admin_ops::job_detail))
        .route("/api/v1/admin/prompts", get(admin_ops::list_prompts))
        .route(
            "/api/v1/admin/sensitive-words",
            get(admin_ops::list_words).post(admin_ops::add_word),
        )
        .route(
            "/api/v1/admin/sensitive-words/{id}",
            delete(admin_ops::remove_word),
        )
        .route(
            "/api/v1/admin/prompts/{code}/status",
            post(admin_ops::set_prompt_status),
        )
        .with_state(state)
        .fallback(static_files::fallback)
}

/// 给负载均衡与容器探针用的最小健康检查，不碰数据库。
async fn healthz() -> &'static str {
    "ok"
}

#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
    database: &'static str,
}

async fn health(State(state): State<AppState>) -> Json<ApiBody<Health>> {
    let probe = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await;
    let database = if probe.is_ok() { "ok" } else { "down" };

    Json(ApiBody::ok(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        database,
    }))
}
