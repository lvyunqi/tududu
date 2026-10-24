use std::path::{Path, PathBuf};

/// 本地磁盘存储。所有路径都是相对数据目录的 key，不接受绝对路径。
#[derive(Debug, Clone)]
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 把相对 key 拼成绝对路径，同时挡掉包含 `..` 的 key。
    fn resolve(&self, key: &str) -> std::io::Result<PathBuf> {
        if key.is_empty() || key.contains("..") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "非法的存储 key",
            ));
        }

        Ok(self.root.join(key))
    }

    pub async fn save(&self, key: &str, bytes: &[u8]) -> std::io::Result<()> {
        let path = self.resolve(key)?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, bytes).await
    }

    pub async fn read(&self, key: &str) -> std::io::Result<Vec<u8>> {
        tokio::fs::read(self.resolve(key)?).await
    }

    pub async fn remove(&self, key: &str) -> std::io::Result<()> {
        match tokio::fs::remove_file(self.resolve(key)?).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_segments() {
        let store = LocalStore::new(PathBuf::from("/tmp/data"));
        assert!(store.resolve("../etc/passwd").is_err());
        assert!(store.resolve("").is_err());
        assert!(store.resolve("u/abc/2026/10/x.png").is_ok());
    }
}
