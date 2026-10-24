//! 图片资产的本地存储。图片落在数据目录，库里只存相对路径与元信息。

pub mod local_store;
pub mod store;

use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::storage::local_store::LocalStore;

/// 单张图片上限 20MB。
pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

/// 只按文件头判断类型，不信客户端给的 Content-Type。
pub fn detect_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some(("image/png", "png"));
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(("image/jpeg", "jpg"));
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(("image/webp", "webp"));
    }

    None
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// 保存一次上传。同一用户重复上传相同内容时直接返回已有资产。
pub async fn save_upload(
    db: &PgPool,
    files: &LocalStore,
    owner: Uuid,
    short_code: &str,
    bytes: &[u8],
) -> Result<store::AssetRow, ApiError> {
    if bytes.is_empty() {
        return Err(ApiError::bad_request("文件是空的"));
    }

    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::bad_request("单张图片不能超过 20MB"));
    }

    let now = Utc::now();
    let prefix = format!("u/{short_code}/{}/{}", now.format("%Y"), now.format("%m"));
    store_asset(db, files, owner, &prefix, bytes, "upload").await
}

/// 保存模型生成的结果图。和上传一样按内容去重，同一张图不会重复占盘。
pub async fn save_generated(
    db: &PgPool,
    files: &LocalStore,
    owner: Uuid,
    bytes: &[u8],
) -> Result<store::AssetRow, ApiError> {
    if bytes.is_empty() {
        return Err(ApiError::internal("上游返回的图片是空的"));
    }

    let now = Utc::now();
    let prefix = format!("g/{owner}/{}/{}", now.format("%Y"), now.format("%m"));
    store_asset(db, files, owner, &prefix, bytes, "job").await
}

async fn store_asset(
    db: &PgPool,
    files: &LocalStore,
    owner: Uuid,
    prefix: &str,
    bytes: &[u8],
    source: &str,
) -> Result<store::AssetRow, ApiError> {
    let (mime, ext) =
        detect_image(bytes).ok_or_else(|| ApiError::bad_request("只支持 PNG、JPEG、WebP"))?;

    let sha256 = sha256_hex(bytes);
    if let Some(existing) = store::find_by_sha(db, owner, &sha256).await? {
        return Ok(existing);
    }

    let object_key = format!("{prefix}/{}.{}", Uuid::new_v4().simple(), ext);

    files.save(&object_key, bytes).await.map_err(|err| {
        tracing::error!(error = %err, key = %object_key, "写入本地文件失败");
        ApiError::internal("保存图片失败")
    })?;

    let new_asset = store::NewAsset {
        owner_user_id: owner,
        object_key: &object_key,
        mime,
        byte_size: bytes.len() as i64,
        sha256: &sha256,
        source,
    };

    match store::insert(db, &new_asset).await {
        Ok(row) => Ok(row),
        Err(err) => {
            // 入库失败就把刚写的文件删掉，别留孤儿文件
            let _ = files.remove(&object_key).await;
            Err(err.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_known_image_headers() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_image(&png), Some(("image/png", "png")));

        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image(&jpeg), Some(("image/jpeg", "jpg")));

        let webp = b"RIFF____WEBP".to_vec();
        assert_eq!(detect_image(&webp), Some(("image/webp", "webp")));
    }

    #[test]
    fn rejects_other_payloads() {
        assert!(detect_image(b"GIF89a").is_none());
        assert!(detect_image(b"<html></html>").is_none());
        assert!(detect_image(b"").is_none());
    }

    #[test]
    fn sha256_is_stable() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
