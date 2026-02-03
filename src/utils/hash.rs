use anyhow::Context;
use argon2::Argon2;
use argon2::PasswordVerifier;
use argon2::password_hash::PasswordHasher;

use anyhow::Result;
use rand::RngCore;
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

pub fn hash_ip(ip: &IpAddr, user_agent: &str, daily_salt: [u8; 16], entity_id: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(ip.to_string().as_bytes());
    hasher.update(user_agent.as_bytes());
    hasher.update(&daily_salt);
    hasher.update(entity_id.as_bytes());
    let hash = hasher.finalize();
    hex::encode(hash.as_bytes())[..32].to_string()
}

pub fn visitor_id() -> String {
    // random 32 byte hex string
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bs58::encode(bytes).into_string()
}

pub fn session_token() -> String {
    // random 32 byte hex string
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bs58::encode(bytes).into_string()
}

pub fn onboarding_token() -> String {
    let mut bytes = [0u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    bs58::encode(bytes).into_string()
}

pub fn db_name() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bs58::encode(bytes).into_string()
}
