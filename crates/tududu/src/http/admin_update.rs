//! 后台在线升级：查版本、下载替换二进制、申请重启。
//!
//! 和命令行 update install 走同一条路：下载 → 校验 SHA-256 → 覆盖当前程序。
//! 覆盖完进程还跑着旧代码，所以要重启才生效，这里只做「装好了」和「申请重启」两件事。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;

use crate::error::{ApiBody, ApiError};
use crate::http::routes::{AppState, build_commit};
use crate::update::UpdateService;

/// 安装了新版本、等着重启时记在这里，界面上要提示。
#[derive(Debug, Default)]
pub struct PendingUpgrade {
    pub version: Option<String>,
    pub installed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub type UpgradeSlot = Arc<std::sync::Mutex<PendingUpgrade>>;

#[derive(Debug, Serialize)]
pub struct UpdateStatus {
    pub current: String,
    pub latest: String,
    pub has_update: bool,
    pub notes: String,
    pub size: i64,
    pub published_at: String,
    /// 有没有已经装好、等重启的新版本
    pub pending_version: Option<String>,
    /// 能不能自助安装（容器里、没写权限时为 false）
    pub can_install: bool,
}

/// 查后台自己的版本情况。
pub async fn status(State(state): State<AppState>) -> Result<Json<ApiBody<UpdateStatus>>, ApiError> {
    let current = crate::update::build_version();

    let info = state
        .updates
        .version("linux", &current, &build_commit())
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "后台查版本失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40004, "暂时查不到版本信息")
        })?;

    let pending = state
        .upgrade
        .lock()
        .map(|slot| slot.version.clone())
        .unwrap_or_default();

    Ok(Json(ApiBody::ok(UpdateStatus {
        current,
        latest: info.latest,
        // 已经装好待重启的，就不再提示「有新版本」
        has_update: info.has_update && pending.is_none(),
        notes: info.notes,
        size: info.size,
        published_at: info.published_at,
        pending_version: pending,
        can_install: can_install(),
    })))
}

#[derive(Debug, Serialize)]
pub struct InstallResult {
    pub version: String,
    pub restart_required: bool,
}

/// 下载并替换当前二进制，装完需要重启才生效。
pub async fn install(State(state): State<AppState>) -> Result<Json<ApiBody<InstallResult>>, ApiError> {
    if !can_install() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            40005,
            "当前环境不能自助安装（容器部署或程序目录不可写）",
        ));
    }

    let current = crate::update::build_version();
    let info = state
        .updates
        .version("linux", &current, &build_commit())
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "安装前查版本失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40004, "暂时查不到版本信息")
        })?;

    if !info.has_update {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, 40006, "已经是最新版本"));
    }

    let release = state.updates.latest().await.map_err(|err| {
        tracing::warn!(error = %err, "安装时取 release 失败");
        ApiError::new(StatusCode::BAD_GATEWAY, 40004, "暂时拿不到安装包")
    })?;

    let asset = UpdateService::asset_for(&release, "linux")
        .ok_or_else(|| ApiError::not_found("这个版本没有服务端安装包"))?;

    let bytes = state
        .http
        .get(&asset.browser_download_url)
        .header("User-Agent", "tududu-update")
        .send()
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "下载安装包失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40007, "下载安装包失败")
        })?
        .bytes()
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "读取安装包失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40007, "下载安装包失败")
        })?;

    // 有校验和就必须对上，对不上不动磁盘
    if let Some(expected) = info.sha256.as_deref() {
        use sha2::{Digest, Sha256};

        let actual = hex::encode(Sha256::digest(&bytes));
        if !actual.eq_ignore_ascii_case(expected) {
            tracing::error!(expected, actual, "安装包校验不通过");
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                40008,
                "安装包校验不通过，已放弃安装",
            ));
        }
    }

    let path = std::env::current_exe().map_err(|err| {
        tracing::warn!(error = %err, "拿不到程序路径");
        ApiError::internal("拿不到程序路径")
    })?;

    replace(&path, &bytes).map_err(|err| {
        tracing::warn!(error = %err, "替换程序失败");
        ApiError::internal("替换程序失败，详情见服务端日志")
    })?;

    if let Ok(mut slot) = state.upgrade.lock() {
        slot.version = Some(info.latest.clone());
        slot.installed_at = Some(chrono::Utc::now());
    }

    tracing::info!(version = %info.latest, "自助升级已安装，等待重启");

    Ok(Json(ApiBody::ok(InstallResult {
        version: info.latest,
        restart_required: true,
    })))
}

/// 重启当前进程：以 75 退出，交给 systemd / 启动器拉起来。
pub async fn restart(State(_state): State<AppState>) -> Result<Json<ApiBody<&'static str>>, ApiError> {
    tracing::info!("后台申请重启");

    // 先把响应发出去，再退出
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        tracing::info!("以退出码 75 结束进程，等待托管方重启");
        std::process::exit(75);
    });

    Ok(Json(ApiBody::ok("正在重启")))
}

/// 能不能就地替换：容器里不做（更新靠换镜像），程序目录写不了也不做。
fn can_install() -> bool {
    if std::env::var("TUDUDU_DISABLE_SELF_UPDATE")
        .is_ok_and(|value| value == "1" || value == "true")
    {
        return false;
    }

    // 容器里换二进制没意义，重启就回到镜像里的版本
    if std::path::Path::new("/.dockerenv").exists() {
        return false;
    }

    let Ok(path) = std::env::current_exe() else {
        return false;
    };
    let Some(dir) = path.parent() else {
        return false;
    };

    std::fs::metadata(dir)
        .map(|meta| !meta.permissions().readonly())
        .unwrap_or(false)
}

/// 临时文件写完再原子替换，旧程序留一份 backup。
fn replace(current: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let dir = current
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "拿不到程序所在目录"))?;

    let name = current
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("tududu");
    std::fs::copy(current, dir.join(format!("{name}-backup")))?;

    let temp = dir.join(".tududu-update.tmp");
    let mut file = std::fs::File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755))?;
    }

    std::fs::rename(&temp, current)
}
