//! 移动端外壳。界面由 web 构建产物提供，这里只负责承载。

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![open_external])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
