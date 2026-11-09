//! 移动端外壳。界面由 web 构建产物提供，这里只负责承载。

use tauri::Manager;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![open_external, app_build_info])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
