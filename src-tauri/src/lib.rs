//! 移动端外壳。界面由 web 构建产物提供，这里只负责承载。

use futures_util::StreamExt;
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

/// 用系统浏览器打开一个链接。
///
/// App 内 WebView 的 blob 图片既不能长按保存，也不会接管 a[download]，
/// 所以下载走这条路：后端给出短时签名直链，交给浏览器自己保存。
#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|err| err.to_string())
}

/// 下载更新包，边下边往界面上报进度，返回本地文件路径。
#[tauri::command]
async fn download_update(
    app: tauri::AppHandle,
    url: String,
    file_name: String,
) -> Result<String, String> {
    use std::io::Write;

    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|err| format!("拿不到缓存目录：{err}"))?;

    std::fs::create_dir_all(&dir).map_err(|err| format!("创建缓存目录失败：{err}"))?;
    let path = dir.join(file_name);

    let response = reqwest::get(&url)
        .await
        .map_err(|err| format!("下载失败：{err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "下载失败，服务端返回 {}（{url}）",
            response.status()
        ));
    }

    let total = response.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(&path).map_err(|err| format!("创建文件失败：{err}"))?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("下载中断：{err}"))?;
        file.write_all(&chunk)
            .map_err(|err| format!("写文件失败：{err}"))?;
        downloaded += chunk.len() as u64;

        // 每块都报一次，界面那边自己节流
        let _ = app.emit(
            "update://progress",
            serde_json::json!({ "downloaded": downloaded, "total": total }),
        );
    }

    file.flush().map_err(|err| format!("写文件失败：{err}"))?;

    Ok(path.to_string_lossy().to_string())
}

/// 当前构建的版本与提交号，前端拿去问服务端有没有更新。
///
/// 版本取打包时写进配置的版本；提交号由流水线注入，没有就是空。
#[tauri::command]
fn app_build_info(app: tauri::AppHandle) -> serde_json::Value {
    serde_json::json!({
        "version": app.package_info().version.to_string(),
        "commit": option_env!("TUDUDU_BUILD_COMMIT").unwrap_or(""),
    })
}

/// 拉起系统安装界面。安卓上交给系统解析 apk，桌面上直接打开文件。
#[tauri::command]
fn install_update(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|err| err.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_external,
            download_update,
            install_update,
            app_build_info
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
