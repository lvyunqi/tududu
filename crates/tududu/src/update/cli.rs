//! 服务端自助更新：查版本、下载、校验、替换二进制。
//!
//! 走自己服务端的中转地址，不直连 GitHub；替换完以退出码 75 通知启动器重启。

use std::io::Write;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

const RESTART_CODE: i32 = 75;

/// 更新相关的参数。
#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub base_url: String,
    /// 直接装，不再问一句
    pub yes: bool,
}

/// 默认从本机服务查，部署在别处时用 --url 指过去。
pub fn default_base() -> String {
    std::env::var("TUDUDU_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned())
}

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// 查一下有没有新版本，打印结果。
pub async fn check(options: &UpdateOptions) -> anyhow::Result<Option<serde_json::Value>> {
    let current = current_version();
    let url = format!(
        "{}/api/v1/app/version?platform=linux&current={}",
        options.base_url.trim_end_matches('/'),
        current
    );

    let body: serde_json::Value = reqwest::get(&url).await?.json().await?;
    let info = body
        .get("data")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("服务端返回格式不对"))?;

    let latest = info
        .get("latest")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    println!("当前版本：{current}");
    println!("最新版本：{latest}");

    let has_update = info
        .get("has_update")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if !has_update {
        println!("已经是最新的");
        return Ok(None);
    }

    if let Some(notes) = info.get("notes").and_then(|value| value.as_str())
        && !notes.trim().is_empty()
    {
        println!("\n更新说明：\n{notes}");
    }

    Ok(Some(info))
}

/// 下载并替换当前二进制。替换成功后返回退出码，由调用方决定怎么重启。
pub async fn install(options: &UpdateOptions) -> anyhow::Result<i32> {
    let Some(info) = check(options).await? else {
        return Ok(0);
    };

    if !options.yes {
        print!("\n确认安装？输入 y 继续：");
        std::io::stdout().flush()?;

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            println!("已取消");
            return Ok(0);
        }
    }

    let path = info
        .get("download_url")
        .and_then(|value| value.as_str())
        .unwrap_or("/api/v1/app/download?platform=linux");

    let url = if path.starts_with("http") {
        path.to_owned()
    } else {
        format!("{}{}", options.base_url.trim_end_matches('/'), path)
    };

    println!("下载中…");
    let bytes = reqwest::get(&url).await?.bytes().await?;
    println!("已下载 {} 字节", bytes.len());

    // 服务端给了校验和就必须核对上，不然不换
    if let Some(expected) = info.get("sha256").and_then(|value| value.as_str()) {
        let actual = hex::encode(Sha256::digest(&bytes));
        if !actual.eq_ignore_ascii_case(expected) {
            anyhow::bail!("校验失败：期望 {expected}，实际 {actual}");
        }
        println!("校验通过");
    } else {
        println!("服务端没有提供校验和，跳过校验");
    }

    let current = std::env::current_exe()?;
    replace(&current, &bytes)?;

    println!("已安装新版本，重启后生效");
    Ok(RESTART_CODE)
}

/// 先写临时文件、再原子替换，旧程序留一份 backup。
fn replace(current: &PathBuf, bytes: &[u8]) -> anyhow::Result<()> {
    let dir = current
        .parent()
        .ok_or_else(|| anyhow::anyhow!("拿不到程序目录"))?;

    let backup = dir.join(format!(
        "{}-backup",
        current
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("tududu")
    ));
    std::fs::copy(current, &backup)?;

    let temp = dir.join(".tududu-update.tmp");
    std::fs::write(&temp, bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755))?;
    }

    std::fs::rename(&temp, current)?;
    println!("旧版本已备份到 {}", backup.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_falls_back_to_localhost() {
        // 环境变量没设时应该是本机
        if std::env::var("TUDUDU_BASE_URL").is_err() {
            assert_eq!(default_base(), "http://127.0.0.1:8080");
        }
    }

    #[test]
    fn restart_code_is_75() {
        // 启动器认这个退出码来重启
        assert_eq!(RESTART_CODE, 75);
    }
}
