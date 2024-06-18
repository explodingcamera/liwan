use eyre::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

pub const MAX_SESSION_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 14);

#[derive(Debug, Serialize, Deserialize)]
pub enum MaxMindEdition {
    #[serde(rename = "GeoLite2-City")]
    GeoLite2City,
    #[serde(rename = "GeoLite2-Country")]
    GeoLite2Country,
    #[serde(rename = "GeoLite2-ASN")]
    GeoLite2ASN,
    #[serde(rename = "GeoIP2-City")]
    GeoIP2City,
    #[serde(rename = "GeoIP2-Country")]
    GeoIP2Country,
}

impl TryFrom<&str> for MaxMindEdition {
    type Error = eyre::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GeoLite2-City" => Ok(Self::GeoLite2City),
            "GeoLite2-Country" => Ok(Self::GeoLite2Country),
            "GeoLite2-ASN" => Ok(Self::GeoLite2ASN),
            "GeoIP2-City" => Ok(Self::GeoIP2City),
            "GeoIP2-Country" => Ok(Self::GeoIP2Country),
            _ => bail!("Invalid MaxMind edition"),
        }
    }
}

fn default_db_path() -> String {
    "liwan.db".to_string()
}

fn default_port() -> u16 {
    3000
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_db_path")]
    pub db_path: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub geoip: Option<GeoIpConfig>,

    #[serde(default, rename = "user")]
    pub users: Vec<User>,

    #[serde(default, rename = "group")]
    pub groups: Vec<Group>,

    #[serde(default, rename = "entity")]
    pub entities: Vec<Entity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoIpConfig {
    pub maxmind_db_path: Option<String>,
    pub maxmind_account_id: Option<String>,
    pub maxmind_license_key: Option<String>,
    pub maxmind_edition: Option<MaxMindEdition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub display_name: String,
    pub entities: Vec<String>,

    #[serde(default)]
    pub public: bool,
    pub password: Option<String>, // enable public access with password protection
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub groups: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "readonly")]
    ReadOnly,
}

static DEFAULT_CONFIG: &str = include_str!("../config.example.toml");

impl Config {
    pub fn from_file(file: impl AsRef<Path>) -> Result<Self> {
        if std::fs::metadata(file.as_ref()).is_err() {
            std::fs::write("liwan.config.toml", DEFAULT_CONFIG)
                .wrap_err("Failed to write default config file")?;

            println!("Config file not found, default config written to {}", file.as_ref().display());
        }

        let config = std::fs::read_to_string(file).wrap_err("Failed to read config file")?;
        toml::from_str(&config).wrap_err("Failed to parse config file")
    }
}
