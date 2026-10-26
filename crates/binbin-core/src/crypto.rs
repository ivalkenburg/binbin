use crate::error::{Error, Result, bad};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};

pub fn secret() -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}
pub fn digest(b: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(b.as_ref()))
}
pub fn hash_password(p: &str) -> Result<String> {
    if p.len() < 12 || p.len() > 1024 {
        return Err(bad("Passwords must contain 12–1024 bytes"));
    }
    Argon2::default()
        .hash_password(p.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|p| p.to_string())
        .map_err(|_| Error(500, "Password hashing failed".into()))
}
pub fn verify_password(p: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .is_ok_and(|h| Argon2::default().verify_password(p.as_bytes(), &h).is_ok())
}
