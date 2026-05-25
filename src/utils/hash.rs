use anyhow::Context;
use argon2::Argon2;
use argon2::PasswordVerifier;
use argon2::password_hash::PasswordHasher;

use anyhow::Result;
use rand::distr::Alphanumeric;
use rand::distr::SampleString;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn hash_password(password: &str) -> Result<String> {
    let hash = Argon2::default().hash_password(password.as_bytes()).context("Failed to hash password")?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let hash = argon2::PasswordHash::new(hash).context("Invalid hash")?;
    let argon2 = Argon2::default();
    argon2.verify_password(password.as_bytes(), &hash).context("Failed to verify password")
}

pub fn visitor_group_id(ip: &IpAddr, user_agent: &str, daily_salt: &str, entity_id: &str) -> String {
    hash_visitor_group(&[ip.to_string().as_bytes(), user_agent.as_bytes(), daily_salt.as_bytes(), entity_id.as_bytes()])
}

pub fn visitor_group_id_cidr(
    ip: &IpAddr,
    ipv4_prefix: u8,
    ipv6_prefix: u8,
    daily_salt: &str,
    entity_id: &str,
) -> String {
    let masked_ip = match ip {
        IpAddr::V4(ip) => IpAddr::V4(mask_ipv4(*ip, ipv4_prefix)),
        IpAddr::V6(ip) => IpAddr::V6(mask_ipv6(*ip, ipv6_prefix)),
    };
    hash_visitor_group(&[masked_ip.to_string().as_bytes(), daily_salt.as_bytes(), entity_id.as_bytes()])
}

fn hash_visitor_group(parts: &[&[u8]]) -> String {
    const CHARS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    let hash = hasher.finalize();

    let mut result = String::with_capacity(16);
    for byte in hash.as_bytes().iter().take(16) {
        result.push(CHARS[(byte % 62) as usize] as char);
    }
    result
}

fn mask_ipv4(ip: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    let prefix = prefix.min(32);
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
    Ipv4Addr::from(u32::from(ip) & mask)
}

fn mask_ipv6(ip: Ipv6Addr, prefix: u8) -> Ipv6Addr {
    let prefix = prefix.min(128);
    let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
    Ipv6Addr::from(u128::from(ip) & mask)
}

pub fn visitor_group_id_fallback() -> String {
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
