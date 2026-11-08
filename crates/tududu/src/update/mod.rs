//! 版本更新：从 GitHub Releases 取最新包，并提供下载中转。

pub mod cli;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// 缓存多久重新问一次 GitHub，避免每次启动都去打人家接口。
const CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    #[serde(default)]
    pub size: i64,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    /// 最新版本号，去掉前缀 v
    pub latest: String,
    /// 客户端当前版本，没传就是空
    pub current: String,
    /// 客户端该不该更新
    pub has_update: bool,
    pub notes: String,
    pub published_at: String,
    /// 对应平台的包大小，找不到是 0
    pub size: i64,
    /// 站内中转下载地址
    pub download_url: String,
    /// 安装包的 SHA-256，有才校验
    pub sha256: Option<String>,
    /// 这次构建对应的提交，同一个版本号重新发布时靠它区分
    pub build_commit: Option<String>,
}

/// release 里附带的构建信息，发行包每次构建都会重新生成。
#[derive(Debug, Clone, Default)]
pub struct BuildInfo {
    pub version: String,
    pub commit: String,
    pub android_sha256: String,
    pub linux_sha256: String,
}

#[derive(Clone)]
pub struct UpdateService {
    http: reqwest::Client,
    repo: String,
    cache: Arc<RwLock<Option<(Instant, Release)>>>,
    build_cache: Arc<RwLock<Option<(Instant, BuildInfo)>>>,
}

impl UpdateService {
    pub fn new(repo: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();

        Self {
            http,
            repo,
            cache: Arc::new(RwLock::new(None)),
            build_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// 取最新 release，带 5 分钟缓存；refresh 为真时跳过缓存。
    pub async fn latest(&self) -> Result<Release, String> {
        self.latest_fresh(false).await
    }

    pub async fn latest_fresh(&self, refresh: bool) -> Result<Release, String> {
        if !refresh
            && let Some((at, release)) = self.cache.read().await.as_ref()
            && at.elapsed() < CACHE_TTL
        {
            return Ok(release.clone());
        }

        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let response = self
            .http
            .get(&url)
            .header("User-Agent", "tududu-update")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|err| format!("请求 GitHub 失败：{err}"))?;

        if !response.status().is_success() {
            return Err(format!("GitHub 返回 {}", response.status()));
        }

        let release: Release = response
            .json()
            .await
            .map_err(|err| format!("解析 release 失败：{err}"))?;

        *self.cache.write().await = Some((Instant::now(), release.clone()));
        Ok(release)
    }

    /// 某个平台对应的包：安卓取 apk，其它取 linux 二进制。
    pub fn asset_for<'a>(release: &'a Release, platform: &str) -> Option<&'a ReleaseAsset> {
        let want_apk = platform == "android";

        release.assets.iter().find(|item| {
            let name = item.name.to_lowercase();
            if want_apk {
                name.ends_with(".apk")
            } else {
                name.contains("linux")
            }
        })
    }

    /// 取并缓存 BUILD_INFO.txt。
    ///
    /// 同一个标签可能被反复覆盖发布，光看版本号分不出新旧，
    /// 所以额外发一份构建信息，里面带提交号和每个包的哈希。
    pub async fn build_info(&self, release: &Release) -> Option<BuildInfo> {
        if let Some((at, info)) = self.build_cache.read().await.as_ref()
            && at.elapsed() < CACHE_TTL
        {
            return Some(info.clone());
        }

        let asset = release
            .assets
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case("BUILD_INFO.txt"))?;

        let text = self
            .http
            .get(&asset.browser_download_url)
            .header("User-Agent", "tududu-update")
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;

        let mut info = BuildInfo::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().to_owned();
            match key.trim() {
                "version" => info.version = value,
                "commit" => info.commit = value,
                "android_sha256" => info.android_sha256 = value,
                "linux_sha256" => info.linux_sha256 = value,
                _ => {}
            }
        }

        *self.build_cache.write().await = Some((Instant::now(), info.clone()));
        Some(info)
    }

    /// 从 SHA256SUMS.txt 里找出某个包的校验和。找不到就返回 None，不阻塞更新。
    pub async fn checksum_for(&self, release: &Release, asset_name: &str) -> Option<String> {
        let sums = release
            .assets
            .iter()
            .find(|item| item.name.to_uppercase().contains("SHA256"))?;

        let text = self
            .http
            .get(&sums.browser_download_url)
            .header("User-Agent", "tududu-update")
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;

        text.lines().find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');

            (name == asset_name).then(|| hash.to_lowercase())
        })
    }

    /// 查版本。除了比版本号，同版本还要比构建哈希：
    /// 覆盖发布同一个标签时版本号没变，得靠构建信息区分新旧。
    pub async fn version(
        &self,
        platform: &str,
        current: &str,
        current_commit: &str,
    ) -> Result<VersionInfo, String> {
        self.version_fresh(platform, current, current_commit, false)
            .await
    }

    pub async fn version_fresh(
        &self,
        platform: &str,
        current: &str,
        current_commit: &str,
        refresh: bool,
    ) -> Result<VersionInfo, String> {
        let release = self.latest_fresh(refresh).await?;
        let latest = normalize(&release.tag_name);
        let asset = Self::asset_for(&release, platform);
        let info = self.build_info(&release).await;

        let sha256 = match (&info, asset) {
            (Some(build), Some(item)) => {
                let from_build = if platform == "android" {
                    &build.android_sha256
                } else {
                    &build.linux_sha256
                };

                if from_build.is_empty() {
                    self.checksum_for(&release, &item.name).await
                } else {
                    Some(from_build.clone())
                }
            }
            (None, Some(item)) => self.checksum_for(&release, &item.name).await,
            _ => None,
        };

        let same_version = latest == normalize(current);
        let rebuilt = match &info {
            Some(build) if !build.commit.is_empty() && !current_commit.is_empty() => {
                build.commit != current_commit
            }
            _ => false,
        };

        Ok(VersionInfo {
            // 版本号更高要更新；版本号相同但构建对不上，说明这个标签被重新发过，也要更新
            has_update: is_newer(&latest, current) || (same_version && rebuilt),
            latest,
            current: normalize(current),
            notes: release.body.clone(),
            published_at: release.published_at.clone(),
            size: asset.map(|item| item.size).unwrap_or(0),
            download_url: format!("/api/v1/app/download?platform={platform}"),
            sha256,
            build_commit: info.as_ref().map(|build| build.commit.clone()),
        })
    }
}

/// 当前程序的版本号。
///
/// 发版流水线会用 TUDUDU_BUILD_VERSION 注入标签里的版本，
/// 仓库里 Cargo.toml 的版本只在本地开发时兜底，免得两边对不上。
pub fn build_version() -> String {
    let raw = option_env!("TUDUDU_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    normalize(raw)
}

/// 去掉版本号前面的 v。
pub fn normalize(version: &str) -> String {
    version.trim().trim_start_matches(['v', 'V']).to_owned()
}

/// 比较两个版本号，前者比后者新就返回 true。只认数字段，其它一律当分隔符。
pub fn is_newer(latest: &str, current: &str) -> bool {
    let left = parse(latest);
    let right = parse(current);

    if right.is_empty() {
        return !left.is_empty();
    }

    let len = left.len().max(right.len());
    for index in 0..len {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);

        if a != b {
            return a > b;
        }
    }

    false
}

fn parse(version: &str) -> Vec<u32> {
    normalize(version)
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_versions_are_detected() {
        assert!(is_newer("0.1.14", "0.1.13"));
        assert!(is_newer("0.2.0", "0.1.99"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.13", "0.1.13"));
        assert!(!is_newer("0.1.9", "0.1.13"));
    }

    #[test]
    fn v_prefix_and_suffix_are_ignored() {
        assert!(is_newer("v0.1.14", "0.1.13"));
        assert!(!is_newer("v0.1.13", "0.1.13"));
        assert!(is_newer("0.1.14-rc1", "0.1.13"));
    }

    #[test]
    fn empty_current_means_always_update() {
        assert!(is_newer("0.1.13", ""));
        assert!(!is_newer("", "0.1.13"));
    }

    #[test]
    fn same_version_rebuild_is_detected_by_commit() {
        // 覆盖发布时版本号不变，提交对不上就该更新
        let build = BuildInfo {
            version: "0.1.13".to_owned(),
            commit: "new-sha".to_owned(),
            ..Default::default()
        };

        let same_version = normalize("v0.1.13") == normalize("0.1.13");
        let rebuilt = build.commit != "old-sha";

        assert!(same_version && rebuilt);
        assert!(!(same_version && build.commit != "new-sha"));
    }

    #[test]
    fn picks_apk_and_linux_assets() {
        let release = Release {
            tag_name: "v0.1.14".to_owned(),
            body: String::new(),
            published_at: String::new(),
            assets: vec![
                ReleaseAsset {
                    name: "tududu-v0.1.14-linux-x86_64".to_owned(),
                    size: 10,
                    browser_download_url: "https://example.com/linux".to_owned(),
                },
                ReleaseAsset {
                    name: "tududu-v0.1.14-arm64.apk".to_owned(),
                    size: 20,
                    browser_download_url: "https://example.com/apk".to_owned(),
                },
            ],
        };

        assert_eq!(
            UpdateService::asset_for(&release, "android")
                .map(|item| item.size)
                .unwrap_or_default(),
            20
        );
        assert_eq!(
            UpdateService::asset_for(&release, "linux")
                .map(|item| item.size)
                .unwrap_or_default(),
            10
        );
    }
}
