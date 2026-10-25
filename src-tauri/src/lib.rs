//! 移动端外壳。界面由 web 构建产物提供，这里只负责承载。

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("启动失败");
}
