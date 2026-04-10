use anyhow::Context;
use argon2::Argon2;
use argon2::PasswordVerifier;
use argon2::password_hash::PasswordHasher;

use anyhow::Result;
use rand::distr::Alphanumeric;
use rand::distr::SampleString;
use std::net::IpAddr;

pub fn hash_password(password: &str) -> Result<String> {
    let hash = Argon2::default().hash_password(password.as_bytes()).context("Failed to hash password")?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let hash = argon2::PasswordHash::new(hash).context("Invalid hash")?;
    let argon2 = Argon2::default();
    argon2.verify_password(password.as_bytes(), &hash).context("Failed to verify password")
}

pub fn visitor_id(ip: &IpAddr, user_agent: &str, daily_salt: &str, entity_id: &str) -> String {
    const CHARS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut hasher = blake3::Hasher::new();
    hasher.update(ip.to_string().as_bytes());
    hasher.update(user_agent.as_bytes());
    hasher.update(daily_salt.as_bytes());
    hasher.update(entity_id.as_bytes());
    let hash = hasher.finalize();

    let mut result = String::with_capacity(16);
    for byte in hash.as_bytes().iter().take(16) {
        result.push(CHARS[(byte % 62) as usize] as char);
    }
    result
}

pub fn visitor_id_fallback() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 16)
}

pub fn session_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

pub fn onboarding_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 8)
}

pub fn db_name() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 16)
}
