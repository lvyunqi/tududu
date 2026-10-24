//! 渠道密钥的加解密。
//!
//! 库里只存密文，格式是 12 字节 nonce 拼接 AES-256-GCM 密文，再整体十六进制编码。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const NONCE_LEN: usize = 12;

/// 用主密钥派生固定的 AES-256 密钥，换主密钥就等于换库，需要重新录入渠道密钥。
pub fn derive_key(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"tududu-provider-key-v1");
    hasher.update(secret.as_bytes());
    hasher.finalize().into()
}

pub fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "初始化加密器失败".to_owned())?;

    // nonce 只要不重复即可，取 UUIDv4 的前 12 字节。
    let raw = Uuid::new_v4().into_bytes();
    let nonce_bytes = &raw[..NONCE_LEN];

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(nonce_bytes), plaintext.as_bytes())
        .map_err(|_| "加密渠道密钥失败".to_owned())?;

    let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    packed.extend_from_slice(nonce_bytes);
    packed.extend_from_slice(&ciphertext);

    Ok(hex::encode(packed))
}

pub fn decrypt(key: &[u8; 32], encoded: &str) -> Result<String, String> {
    let packed = hex::decode(encoded).map_err(|_| "密文不是合法的十六进制".to_owned())?;

    if packed.len() <= NONCE_LEN {
        return Err("密文长度不正确".to_owned());
    }

    let (nonce_bytes, ciphertext) = packed.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "初始化加密器失败".to_owned())?;

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| "解密渠道密钥失败".to_owned())?;

    String::from_utf8(plaintext).map_err(|_| "解密结果不是文本".to_owned())
}

/// 展示用掩码，只保留首尾各四位。
pub fn mask(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 8 {
        return "****".to_owned();
    }

    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}...{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_roundtrip() {
        let key = derive_key("unit-test-secret-for-provider-keys");
        let cipher = encrypt(&key, "sk-test-1234567890").expect("encrypt");

        assert_ne!(cipher, "sk-test-1234567890");
        assert_eq!(decrypt(&key, &cipher).as_deref(), Ok("sk-test-1234567890"));
    }

    #[test]
    fn wrong_key_cannot_decrypt() {
        let key = derive_key("unit-test-secret-for-provider-keys");
        let other = derive_key("another-unit-test-secret-value");
        let cipher = encrypt(&key, "sk-test-1234567890").expect("encrypt");

        assert!(decrypt(&other, &cipher).is_err());
    }

    #[test]
    fn mask_keeps_only_edges() {
        assert_eq!(mask("sk-abcdefghijklmn"), "sk-a...klmn");
        assert_eq!(mask("short"), "****");
    }
}
