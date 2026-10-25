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

const UI_MISSING: &str =
    "界面还没有构建。先在 web 与 admin 目录下执行 pnpm build，再重新编译服务端。";

/// 所有没匹配上路由的请求都会落到这里。
pub async fn fallback(uri: Uri) -> Response {
    let path = uri.path();

    if path.starts_with("/api") || path.starts_with("/pay") {
        return (StatusCode::NOT_FOUND, "接口不存在").into_response();
    }

    if path == "/admin" || path.starts_with("/admin/") {
        let rest = path.trim_start_matches("/admin").trim_start_matches('/');
        return respond(pick_admin(rest));
    }

    let rest = path.trim_start_matches('/');
    respond(pick_web(rest))
}

fn pick_web(rest: &str) -> Option<(String, rust_embed::EmbeddedFile)> {
    pick::<WebAssets>(rest)
}

fn pick_admin(rest: &str) -> Option<(String, rust_embed::EmbeddedFile)> {
    pick::<AdminAssets>(rest)
}

/// 具体文件优先；路径看起来不像文件时回退到 index.html，交给前端路由处理。
///
/// 返回实际命中的文件名：Content-Type 必须按它判断，不能用请求路径。
/// 否则访问 `/` 时路径是空串，会被当成二进制流，浏览器直接下载页面而不是显示。
fn pick<T: RustEmbed>(rest: &str) -> Option<(String, rust_embed::EmbeddedFile)> {
    if rest.is_empty() {
        return T::get("index.html").map(|file| ("index.html".to_owned(), file));
    }

    if let Some(file) = T::get(rest) {
        return Some((rest.to_owned(), file));
    }

    if rest.contains('.') {
        return None;
    }

    T::get("index.html").map(|file| ("index.html".to_owned(), file))
}

fn respond(picked: Option<(String, rust_embed::EmbeddedFile)>) -> Response {
    let Some((name, file)) = picked else {
        return (StatusCode::NOT_FOUND, "文件不存在").into_response();
    };

    if file.data.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, UI_MISSING).into_response();
    }

    let mime = content_type(&name);
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
    fn index_is_served_as_html() {
        // 访问根路径时命中 index.html，类型必须是 html，
        // 否则浏览器会把它当文件下载
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
    }

    #[test]
    fn content_type_covers_frontend_assets() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(
            content_type("assets/a.js"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type("assets/a.css"), "text/css; charset=utf-8");
        assert_eq!(
            content_type("manifest.webmanifest"),
            "application/json; charset=utf-8"
        );
        assert_eq!(content_type("logo.svg"), "image/svg+xml");
        assert_eq!(content_type("unknown.bin"), "application/octet-stream");
    }
}
