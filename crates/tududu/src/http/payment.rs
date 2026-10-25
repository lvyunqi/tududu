use std::collections::BTreeMap;

use axum::Form;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Uri};
use axum::response::Html;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiBody, ApiError};
use crate::http::admin::require_admin;
use crate::http::auth::require_user;
use crate::http::routes::AppState;
use crate::payment::{self, store as pay_store};
use crate::provider::crypto;

pub async fn list_plans(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<Vec<pay_store::PlanRow>>>, ApiError> {
    let rows = payment::list_plans(&state.db).await?;
    Ok(Json(ApiBody::ok(rows)))
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub plan_id: Uuid,
}

pub async fn create_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<ApiBody<payment::CheckoutInfo>>, ApiError> {
    let current = require_user(&state, &headers).await?;

    let info = payment::create_order(
        &state.db,
        &state.provider_key,
        state.live().public_base_url.as_deref(),
        current.user_id,
        req.plan_id,
    )
    .await?;

    Ok(Json(ApiBody::ok(info)))
}

pub async fn order_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_no): Path<String>,
) -> Result<Json<ApiBody<pay_store::OrderRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let found = payment::order_status(&state.db, current.user_id, &order_no).await?;
    Ok(Json(ApiBody::ok(found)))
}

/// 易支付异步通知，GET 与 POST 都可能出现。
pub async fn notify_get(
    State(state): State<AppState>,
    uri: Uri,
    Query(params): Query<BTreeMap<String, String>>,
) -> String {
    let raw = uri.query().unwrap_or_default().to_owned();
    payment::handle_notify(&state.db, &state.provider_key, &raw, params).await
}

pub async fn notify_post(
    State(state): State<AppState>,
    Form(params): Form<BTreeMap<String, String>>,
) -> String {
    let raw = params
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    payment::handle_notify(&state.db, &state.provider_key, &raw, params).await
}

/// 支付完成后浏览器跳回来的落地页。
pub async fn return_page() -> Html<&'static str> {
    Html(RETURN_HTML)
}

const RETURN_HTML: &str = r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>支付结果</title>
<style>
  body { margin:0; min-height:100vh; display:flex; align-items:center; justify-content:center;
         font-family:-apple-system,"PingFang SC","Microsoft YaHei",system-ui,sans-serif;
         background:#f7f3ec; color:#241f1a; }
  .box { text-align:center; padding:32px 24px; }
  h2 { margin:0 0 12px; font-size:22px; }
  p { margin:0 0 24px; color:#726858; line-height:1.7; }
  a { display:inline-block; padding:12px 26px; border-radius:14px; text-decoration:none;
      color:#fff; font-weight:600;
      background:linear-gradient(120deg,#ff9a3d,#ff4d8d 55%,#a855f7); }
</style>
</head>
<body>
  <div class="box">
    <h2>支付已提交</h2>
    <p>到账可能有几秒延迟。<br>回到图嘟嘟即可看到最新余额。</p>
    <a href="/">回到图嘟嘟</a>
  </div>
</body>
</html>"#;

#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub name: String,
    pub amount_fen: i64,
    pub credit: i64,
    #[serde(default)]
    pub bonus: i64,
    #[serde(default)]
    pub sort: i32,
}

pub async fn admin_list_plans(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<Vec<pay_store::PlanRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = pay_store::list_plans(&state.db).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn admin_create_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePlanRequest>,
) -> Result<Json<ApiBody<pay_store::PlanRow>>, ApiError> {
    require_admin(&state, &headers)?;

    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("套餐名称不能为空"));
    }

    if req.amount_fen <= 0 || req.credit <= 0 {
        return Err(ApiError::bad_request("金额和到账数量都需要大于 0"));
    }

    if req.bonus < 0 {
        return Err(ApiError::bad_request("赠送数量不能是负数"));
    }

    let row = pay_store::insert_plan(
        &state.db,
        name,
        req.amount_fen,
        req.credit,
        req.bonus,
        req.sort,
    )
    .await?;

    Ok(Json(ApiBody::ok(row)))
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub gateway_url: String,
    pub pid: String,
    pub key: String,
    #[serde(default)]
    pub pay_type: Option<String>,
}

pub async fn admin_list_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiBody<Vec<pay_store::PayChannelRow>>>, ApiError> {
    require_admin(&state, &headers)?;
    let rows = pay_store::list_channels(&state.db).await?;
    Ok(Json(ApiBody::ok(rows)))
}

pub async fn admin_create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ApiBody<pay_store::PayChannelRow>>, ApiError> {
    require_admin(&state, &headers)?;

    let name = req.name.trim();
    let gateway = req.gateway_url.trim().trim_end_matches('/');
    let pid = req.pid.trim();
    let key = req.key.trim();

    if name.is_empty() || pid.is_empty() {
        return Err(ApiError::bad_request("通道名称和商户号不能为空"));
    }

    if !gateway.starts_with("http://") && !gateway.starts_with("https://") {
        return Err(ApiError::bad_request(
            "网关地址需要以 http:// 或 https:// 开头",
        ));
    }

    if key.len() < 8 {
        return Err(ApiError::bad_request("商户密钥长度看起来不对"));
    }

    let cipher = crypto::encrypt(&state.provider_key, key).map_err(|err| {
        tracing::error!(error = %err, "加密支付密钥失败");
        ApiError::internal("保存密钥失败")
    })?;

    let pay_type = req.pay_type.as_deref().unwrap_or("alipay");
    let row = pay_store::insert_channel(&state.db, name, gateway, pid, &cipher, pay_type).await?;

    Ok(Json(ApiBody::ok(row)))
}
