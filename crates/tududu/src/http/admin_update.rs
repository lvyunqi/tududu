//! 后台在线升级：版本状态、下载安装、申请重启。
//!
//! 状态机照着常见发行包的做法来：准备 → 下载 → 校验 → 安装 → 待重启。
//! 前台页面轮询这个状态画进度，装好之后由管理员点重启服务，
//! 进程以 75 退出，交给 systemd 或启动器拉起来。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::error::{ApiBody, ApiError};
use crate::http::routes::{AppState, build_commit};
use crate::update::UpdateService;

/// 支持的安装阶段，前端按这个画文案和转圈。
pub const PHASE_IDLE: &str = "idle";
pub const PHASE_PREPARING: &str = "preparing";
pub const PHASE_DOWNLOADING: &str = "downloading";
pub const PHASE_VERIFYING: &str = "verifying";
pub const PHASE_INSTALLING: &str = "installing";
pub const PHASE_RESTART_REQUIRED: &str = "restart_required";
pub const PHASE_RESTARTING: &str = "restarting";
pub const PHASE_FAILED: &str = "failed";

/// 进程内的升级状态，后台页面轮询它。
#[derive(Debug, Default)]
pub struct SoftwareState {
    pub phase: Option<String>,
    pub error: Option<String>,
    pub pending_version: Option<String>,
    /// 上次检查时间
    pub checked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub blocked: Option<String>,
}

pub type UpgradeSlot = Arc<Mutex<SoftwareState>>;

#[derive(Debug, Serialize)]
pub struct LatestRelease {
    pub version: String,
    pub tag: String,
    pub package_name: String,
    pub notes: String,
    pub size: i64,
    pub published_at: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SoftwareStatus {
    pub version: String,
    pub commit: String,
    /// binary 或 container
    pub deployment: &'static str,
    /// launcher 或 bare，决定重启能不能自己拉起来
    pub launch_mode: &'static str,
    pub install_enabled: bool,
    pub check_enabled: bool,
    pub update_phase: String,
    pub error: Option<String>,
    pub update_available: bool,
    pub latest: Option<LatestRelease>,
    pub pending_version: Option<String>,
    pub checked_at: Option<i64>,
    /// 不能安装的原因码，前端翻译成说明
    pub blocked: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub refresh: Option<bool>,
}

/// 查状态。refresh 为真时强制绕过缓存重新问一次。
pub async fn status(
    State(state): State<AppState>,
    Query(query): Query<StatusQuery>,
) -> Result<Json<ApiBody<SoftwareStatus>>, ApiError> {
    let snapshot = collect(&state, query.refresh.unwrap_or(false)).await?;
    Ok(Json(ApiBody::ok(snapshot)))
}

/// 手动检查更新：刷新缓存并返回状态。
pub async fn check(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<SoftwareStatus>>, ApiError> {
    let snapshot = collect(&state, true).await?;
    Ok(Json(ApiBody::ok(snapshot)))
}

async fn collect(state: &AppState, refresh: bool) -> Result<SoftwareStatus, ApiError> {
    let current = crate::update::build_version();
    let commit = build_commit();

    let info = state
        .updates
        .version_fresh("linux", &current, &commit, refresh)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "后台查版本失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40004, "暂时查不到版本信息")
        })?;

    let release = state.updates.latest_fresh(refresh).await.ok();
    let asset = release
        .as_ref()
        .and_then(|release| UpdateService::asset_for(release, "linux"));

    let mut guard = state.upgrade.lock().unwrap_or_else(|err| err.into_inner());
    if refresh {
        guard.checked_at = Some(chrono::Utc::now());
        guard.blocked = blocked_reason();
    }

    let blocked = guard.blocked.clone();
    let phase = guard.phase.clone().unwrap_or_else(|| PHASE_IDLE.to_owned());

    Ok(SoftwareStatus {
        version: current,
        commit,
        deployment: if in_container() { "container" } else { "binary" },
        launch_mode: if launched_by_launcher() { "launcher" } else { "bare" },
        install_enabled: blocked.is_none(),
        check_enabled: true,
        update_phase: phase,
        error: guard.error.clone(),
        // 刚装完还没重启的，就别再提示有新版
        update_available: info.has_update && guard.pending_version.is_none(),
        latest: Some(LatestRelease {
            version: info.latest.clone(),
            tag: format!("v{}", info.latest),
            package_name: asset.map(|item| item.name.clone()).unwrap_or_default(),
            notes: info.notes.clone(),
            size: info.size,
            published_at: info.published_at.clone(),
            sha256: info.sha256.clone(),
        }),
        pending_version: guard.pending_version.clone(),
        checked_at: guard.checked_at.map(|at| at.timestamp()),
        blocked,
    })
}

fn is_busy(phase: &str) -> bool {
    matches!(
        phase,
        PHASE_PREPARING | PHASE_DOWNLOADING | PHASE_VERIFYING | PHASE_INSTALLING
    )
}

#[derive(Debug, Deserialize, Default)]
pub struct InstallRequest {
    /// 前端看到的目标版本标签，装的时候再核对一次，防止检查完又发了新版
    pub tag: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InstallAccepted {
    pub accepted: bool,
}

/// 开始安装：立即返回，后台任务推进阶段，页面轮询状态。
pub async fn install(
    State(state): State<AppState>,
    Json(request): Json<InstallRequest>,
) -> Result<Json<ApiBody<InstallAccepted>>, ApiError> {
    if let Some(reason) = blocked_reason() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            40005,
            blocked_message(&reason),
        ));
    }

    {
        let guard = state.upgrade.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(phase) = guard.phase.as_deref()
            && is_busy(phase)
        {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                40009,
                "已经有一次安装在进行中",
            ));
        }
    }

    // 核对目标版本，检查完又发新版就让前端重来
    let latest = state
        .updates
        .version_fresh("linux", &crate::update::build_version(), &build_commit(), true)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "安装前查版本失败");
            ApiError::new(StatusCode::BAD_GATEWAY, 40004, "暂时查不到版本信息")
        })?;

    if !latest.has_update {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, 40006, "已经是最新版本"));
    }

    let expected_tag = format!("v{}", latest.latest);
    if let Some(expected) = request.tag.as_deref()
        && expected != expected_tag
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            40010,
            "版本在这期间变了，请重新检查",
        ));
    }

    {
        let mut guard = state.upgrade.lock().unwrap_or_else(|err| err.into_inner());
        guard.phase = Some(PHASE_PREPARING.to_owned());
        guard.error = None;
        guard.blocked = None;
    }

    let slot = Arc::clone(&state.upgrade);
    let updates = state.updates.clone();
    let http = state.http.clone();

    tokio::spawn(async move {
        let result = run_install(&slot, &updates, &http).await;

        let mut guard = slot.lock().unwrap_or_else(|err| err.into_inner());
        match result {
            Ok(version) => {
                guard.phase = Some(PHASE_RESTART_REQUIRED.to_owned());
                guard.pending_version = Some(version);
                guard.error = None;
            }
            Err(err) => {
                tracing::warn!(error = %err, "后台升级失败");
                guard.phase = Some(PHASE_FAILED.to_owned());
                guard.error = Some(err);
            }
        }
    });

    Ok(Json(ApiBody::ok(InstallAccepted { accepted: true })))
}

/// 真正的下载、校验、替换。阶段写进共享状态，页面靠轮询看进度。
async fn run_install(
    slot: &UpgradeSlot,
    updates: &UpdateService,
    http: &reqwest::Client,
) -> Result<String, String> {
    let release = updates.latest_fresh(true).await?;
    let asset = UpdateService::asset_for(&release, "linux")
        .ok_or_else(|| "这个版本没有服务端安装包".to_owned())?
        .clone();

    let version = crate::update::normalize(&release.tag_name);

    set_phase(slot, PHASE_DOWNLOADING);

    let response = http
        .get(&asset.browser_download_url)
        .header("User-Agent", "tududu-update")
        .send()
        .await
        .map_err(|err| format!("下载失败：{err}"))?;

    if !response.status().is_success() {
        return Err(format!("下载失败，上游返回 {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("读取安装包失败：{err}"))?;

    set_phase(slot, PHASE_VERIFYING);

    if let Some(expected) = updates.checksum_for(&release, &asset.name).await {
        use sha2::{Digest, Sha256};

        let actual = hex::encode(Sha256::digest(&bytes));
        if !actual.eq_ignore_ascii_case(&expected) {
            return Err(format!("校验不通过：期望 {expected}，实际 {actual}"));
        }
    }

    set_phase(slot, PHASE_INSTALLING);

    let path = std::env::current_exe().map_err(|err| format!("拿不到程序路径：{err}"))?;
    replace(&path, &bytes).map_err(|err| format!("替换程序失败：{err}"))?;

    tracing::info!(version = %version, "后台升级已安装，等待重启");
    Ok(version)
}

fn set_phase(slot: &UpgradeSlot, phase: &str) {
    if let Ok(mut guard) = slot.lock() {
        guard.phase = Some(phase.to_owned());
    }
}

#[derive(Debug, Serialize)]
pub struct RestartAccepted {
    pub accepted: bool,
}

/// 重启：以 75 退出，交给 systemd 或启动器拉起来。
pub async fn restart(
    State(state): State<AppState>,
) -> Result<Json<ApiBody<RestartAccepted>>, ApiError> {
    {
        let mut guard = state.upgrade.lock().unwrap_or_else(|err| err.into_inner());
        guard.phase = Some(PHASE_RESTARTING.to_owned());
    }

    tracing::info!("后台申请重启");

    tokio::spawn(async {
        // 先把响应发出去再退
        tokio::time::sleep(Duration::from_millis(300)).await;
        tracing::info!("以退出码 75 结束进程，等待托管方重启");
        std::process::exit(75);
    });

    Ok(Json(ApiBody::ok(RestartAccepted { accepted: true })))
}

/// 不能安装的原因，None 表示可以装。
fn blocked_reason() -> Option<String> {
    if std::env::var("TUDUDU_DISABLE_SELF_UPDATE").is_ok_and(|value| value == "1" || value == "true")
    {
        return Some("not_enabled".to_owned());
    }

    if in_container() {
        return Some("container".to_owned());
    }

    let Ok(path) = std::env::current_exe() else {
        return Some("executable_unavailable".to_owned());
    };
    let Some(dir) = path.parent() else {
        return Some("executable_unavailable".to_owned());
    };

    let writable = std::fs::metadata(dir)
        .map(|meta| !meta.permissions().readonly())
        .unwrap_or(false);

    if !writable {
        return Some("host_managed".to_owned());
    }

    None
}

fn blocked_message(reason: &str) -> &'static str {
    match reason {
        "container" => "容器部署不能自助升级，请换镜像后重新部署",
        "not_enabled" => "自助升级已被 TUDUDU_DISABLE_SELF_UPDATE 关掉",
        "host_managed" => "程序目录不可写，请手动替换二进制",
        _ => "当前环境不能自助升级",
    }
}

fn in_container() -> bool {
    std::path::Path::new("/.dockerenv").exists()
}

fn launched_by_launcher() -> bool {
    std::env::var("TUDUDU_LAUNCHER").is_ok_and(|value| value == "1")
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

    // 保留最近一次备份，回滚靠它
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_phases_are_recognized() {
        assert!(is_busy(PHASE_DOWNLOADING));
        assert!(is_busy(PHASE_VERIFYING));
        assert!(!is_busy(PHASE_IDLE));
        assert!(!is_busy(PHASE_RESTART_REQUIRED));
    }

    #[test]
    fn blocked_reason_has_message() {
        for reason in ["container", "not_enabled", "host_managed", "other"] {
            assert!(!blocked_message(reason).is_empty());
        }
    }
}
