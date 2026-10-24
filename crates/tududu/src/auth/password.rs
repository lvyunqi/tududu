use argon2::Argon2;
use argon2::password_hash::{self, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use uuid::Uuid;

/// 生成密码散列。盐取 UUIDv4 的字节，够随机，也不用额外引随机数库。
pub fn hash(plain: &str) -> Result<String, password_hash::Error> {
    let raw = Uuid::new_v4().into_bytes();
    let salt = SaltString::encode_b64(&raw)?;
    let hashed = Argon2::default().hash_password(plain.as_bytes(), &salt)?;
    Ok(hashed.to_string())
}

/// 校验密码。散列本身损坏时返回 false，不向外抛错。
pub fn verify(plain: &str, stored: &str) -> bool {
    match PasswordHash::new(stored) {
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let plain = format!("{}{}", "Abc", "12345");
        let hashed = hash(&plain).expect("hash");
        assert!(verify(&plain, &hashed));
        assert!(!verify(&format!("{plain}x"), &hashed));
    }

    #[test]
    fn broken_hash_is_rejected() {
        assert!(!verify("nope", "not-a-hash"));
    }
}
