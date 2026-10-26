//! 给下载链接签名，让系统浏览器也能取图。
//!
//! App 内 WebView 对 blob 图片既不能长按保存，也不接管 download 属性，
//! 只能把图片交给系统浏览器。浏览器带不了 Authorization 头，所以改用短时有效的签名参数。

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 下载链接的有效期，够用户点开浏览器保存就行。
pub const LINK_TTL_SECS: i64 = 600;

fn compute(secret: &str, asset_id: &str, expires_at: i64) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC 接受任意长度密钥");
    mac.update(asset_id.as_bytes());
    mac.update(b":");
    mac.update(expires_at.to_string().as_bytes());

    hex::encode(mac.finalize().into_bytes())
}

pub fn sign(secret: &str, asset_id: &str, expires_at: i64) -> String {
    compute(secret, asset_id, expires_at)
}

/// 校验签名与有效期。比较用常量时间，避免被逐位试探。
pub fn verify(secret: &str, asset_id: &str, expires_at: i64, provided: &str) -> bool {
    let expected = compute(secret, asset_id, expires_at);

    if expected.len() != provided.len() {
        return false;
    }

    let mut diff = 0u8;
    for (left, right) in expected.bytes().zip(provided.bytes()) {
        diff |= left ^ right;
    }

    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_round_trip() {
        let sig = sign("secret-key", "asset-1", 1_700_000_000);
        assert!(verify("secret-key", "asset-1", 1_700_000_000, &sig));
    }

    #[test]
    fn different_asset_does_not_verify() {
        let sig = sign("secret-key", "asset-1", 1_700_000_000);
        assert!(!verify("secret-key", "asset-2", 1_700_000_000, &sig));
    }

    #[test]
    fn different_expiry_does_not_verify() {
        let sig = sign("secret-key", "asset-1", 1_700_000_000);
        assert!(!verify("secret-key", "asset-1", 1_700_000_001, &sig));
    }
}
