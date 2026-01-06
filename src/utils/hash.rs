use argon2::Argon2;
use argon2::PasswordVerifier;
use argon2::password_hash::{PasswordHasher, SaltString, rand_core};

use anyhow::Result;
use rand::RngCore;
use sha3::Digest;
use std::net::IpAddr;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| anyhow::anyhow!("Failed to hash password"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let hash = argon2::PasswordHash::new(hash).map_err(|_| anyhow::anyhow!("Invalid hash"))?;
    let argon2 = Argon2::default();
    argon2.verify_password(password.as_bytes(), &hash).map_err(|_| anyhow::anyhow!("Failed to verify password"))
}

pub fn generate_salt() -> String {
    SaltString::generate(&mut rand_core::OsRng).to_string()
}

pub fn hash_ip(ip: &IpAddr, user_agent: &str, daily_salt: &str, entity_id: &str) -> String {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(ip.to_string());
    hasher.update(user_agent);
    hasher.update(daily_salt);
    hasher.update(entity_id);
    let hash = hasher.finalize();
    hex::encode(hash)[..32].to_string()
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
