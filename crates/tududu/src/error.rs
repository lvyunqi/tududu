use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// 统一响应包：`code == 0` 表示成功。
#[derive(Debug, Serialize)]
pub struct ApiBody<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiBody<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok".to_owned(),
            data: Some(data),
        }
    }
}

/// 业务错误的统一出口。
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: i32,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: i32, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, 90001, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiBody::<()> {
            code: self.code,
            message: self.message,
            data: None,
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "数据库操作失败");
        Self::internal("服务暂时不可用，请稍后再试")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error_carries_unified_code() {
        let err = ApiError::internal("boom");
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code, 90001);
        assert_eq!(err.message, "boom");
    }

    #[test]
    fn ok_body_uses_zero_code() {
        let body = ApiBody::ok("payload");
        assert_eq!(body.code, 0);
        assert_eq!(body.data, Some("payload"));
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"code\":0"));
    }
}
