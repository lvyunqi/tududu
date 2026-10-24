//! 把前端构建产物打进二进制，一个可执行文件就能把界面也服务出去。
//!
//! 构建产物在 `web/dist` 和 `admin/dist`，仓库里只有占位文件。
//! 只有先跑过前端构建，嵌进去的才是真页面；否则访问会提示界面未构建。

use axum::body::Body;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../web/dist"]
struct WebAssets;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../admin/dist"]
struct AdminAssets;

const UI_MISSING: &str = "界面还没有构建。先在 web 与 admin 目录下执行 pnpm build，再重新编译服务端。";

/// 所有没匹配上路由的请求都会落到这里。
pub async fn fallback(uri: Uri) -> Response {
    let path = uri.path();

    if path.starts_with("/api") || path.starts_with("/pay") {
        return (StatusCode::NOT_FOUND, "接口不存在").into_response();
    }

    if path == "/admin" || path.starts_with("/admin/") {
        let rest = path.trim_start_matches("/admin").trim_start_matches('/');
        return respond(pick_admin(rest), rest);
    }

    let rest = path.trim_start_matches('/');
    respond(pick_web(rest), rest)
}

fn pick_web(rest: &str) -> Option<rust_embed::EmbeddedFile> {
    pick::<WebAssets>(rest)
}

fn pick_admin(rest: &str) -> Option<rust_embed::EmbeddedFile> {
    pick::<AdminAssets>(rest)
}

/// 具体文件优先；路径看起来不像文件时回退到 index.html，交给前端路由处理。
fn pick<T: RustEmbed>(rest: &str) -> Option<rust_embed::EmbeddedFile> {
    if rest.is_empty() {
        return T::get("index.html");
    }

    if let Some(file) = T::get(rest) {
        return Some(file);
    }

    if rest.contains('.') {
        return None;
    }

    T::get("index.html")
}

fn respond(file: Option<rust_embed::EmbeddedFile>, path: &str) -> Response {
    let Some(file) = file else {
        return (StatusCode::NOT_FOUND, "文件不存在").into_response();
    };

    if file.data.is_empty() && path.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, UI_MISSING).into_response();
    }

    let mime = content_type(path);
    let body = Body::from(file.data.into_owned());

    ([(header::CONTENT_TYPE, mime)], body).into_response()
}

fn content_type(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();

    if lower.ends_with(".html") {
        return "text/html; charset=utf-8";
    }
    if lower.ends_with(".js") || lower.ends_with(".mjs") {
        return "text/javascript; charset=utf-8";
    }
    if lower.ends_with(".css") {
        return "text/css; charset=utf-8";
    }
    if lower.ends_with(".json") || lower.ends_with(".webmanifest") {
        return "application/json; charset=utf-8";
    }
    if lower.ends_with(".svg") {
        return "image/svg+xml";
    }
    if lower.ends_with(".png") {
        return "image/png";
    }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        return "image/jpeg";
    }
    if lower.ends_with(".webp") {
        return "image/webp";
    }
    if lower.ends_with(".ico") {
        return "image/x-icon";
    }
    if lower.ends_with(".woff2") {
        return "font/woff2";
    }
    if lower.ends_with(".map") {
        return "application/json; charset=utf-8";
    }

    "application/octet-stream"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_covers_frontend_assets() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("assets/a.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("assets/a.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("manifest.webmanifest"), "application/json; charset=utf-8");
        assert_eq!(content_type("logo.svg"), "image/svg+xml");
        assert_eq!(content_type("unknown.bin"), "application/octet-stream");
    }
}
