//! 把一段文本渲染成二维码。支付页用它给用户截图扫。

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use qrcode::QrCode;
use qrcode::render::svg;
use serde::Deserialize;

use crate::error::ApiError;
use crate::http::auth::require_user;
use crate::http::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct QrRequest {
    pub text: String,
}

pub async fn render(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<QrRequest>,
) -> Result<Response, ApiError> {
    // 只是个渲染接口，但也不能匿名刷
    require_user(&state, &headers).await?;

    let text = req.text.trim();
    if text.is_empty() || text.len() > 2048 {
        return Err(ApiError::bad_request("二维码内容长度不合法"));
    }

    let code = QrCode::new(text.as_bytes())
        .map_err(|_| ApiError::bad_request("内容无法生成二维码"))?;

    let image = code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#241f1a"))
        .light_color(svg::Color("#fffdf9"))
        .build();

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        image,
    )
        .into_response())
}
