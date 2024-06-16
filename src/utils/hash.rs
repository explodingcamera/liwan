use argon2::password_hash::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::{password_hash::SaltString, Argon2};

use eyre::Result;
use rand::rngs::OsRng;
use rand::Rng;
use sha3::Digest;
use std::net::IpAddr;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| eyre::eyre!("Failed to hash password"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let hash = argon2::PasswordHash::new(hash).map_err(|_| eyre::eyre!("Invalid hash"))?;
    let argon2 = Argon2::default();
    argon2.verify_password(password.as_bytes(), &hash).map_err(|_| eyre::eyre!("Failed to verify password"))
}

pub fn generate_salt() -> String {
    SaltString::generate(&mut OsRng).to_string()
}

pub fn hash_ip(ip: &IpAddr, user_agent: &str, daily_salt: &str, entity_id: &str) -> String {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(ip.to_string());
    hasher.update(user_agent);
    hasher.update(daily_salt);
    hasher.update(entity_id);
    let hash = hasher.finalize();
    format!("{:02x}", hash)[..32].to_string()
}

pub fn random_visitor_id() -> String {
    // random 32 byte hex string
    let mut rng = OsRng;
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    bytes.iter().fold(String::new(), |acc, byte| acc + &format!("{:02x}", byte))
}
