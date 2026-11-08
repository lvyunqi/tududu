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
}

#[derive(Clone)]
pub struct UpdateService {
    http: reqwest::Client,
    repo: String,
    cache: Arc<RwLock<Option<(Instant, Release)>>>,
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
        }
    }

    /// 取最新 release，带 5 分钟缓存。
    pub async fn latest(&self) -> Result<Release, String> {
        if let Some((at, release)) = self.cache.read().await.as_ref()
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

    pub async fn version(&self, platform: &str, current: &str) -> Result<VersionInfo, String> {
        let release = self.latest().await?;
        let latest = normalize(&release.tag_name);
        let asset = Self::asset_for(&release, platform);
        let sha256 = match asset {
            Some(item) => self.checksum_for(&release, &item.name).await,
            None => None,
        };

        Ok(VersionInfo {
            has_update: is_newer(&latest, current),
            latest,
            current: normalize(current),
            notes: release.body.clone(),
            published_at: release.published_at.clone(),
            size: asset.map(|item| item.size).unwrap_or(0),
            download_url: format!("/api/v1/app/download?platform={platform}"),
            sha256,
        })
    }
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
