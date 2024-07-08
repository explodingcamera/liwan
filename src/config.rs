use std::str::FromStr;

use eyre::{bail, Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use poem::http::Uri;
use serde::{Deserialize, Serialize};

fn default_base() -> String {
    "http://localhost:8080".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_db_dir() -> String {
    "./liwan-db".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base")]
    pub base_url: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_db_dir")]
    pub db_dir: String,

    pub geoip: Option<GeoIpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpConfig {
    pub maxmind_db_path: Option<String>,
    pub maxmind_account_id: Option<String>,
    pub maxmind_license_key: Option<String>,
    pub maxmind_edition: Option<String>,
}

pub static DEFAULT_CONFIG: &str = include_str!("../config.example.toml");

impl Config {
    pub fn load(path: Option<String>) -> Result<Self> {
        let config: Config = Figment::new()
            .merge(Toml::file(path.unwrap_or("liwan.config.toml".to_string())))
            .merge(Env::prefixed("LIWAN_"))
            .extract()?;

        let url: Uri = Uri::from_str(&config.base_url).wrap_err("Invalid base URL")?;

        if ![Some("http"), Some("https")].contains(&url.scheme_str()) {
            bail!("Invalid base URL: protocol must be either http or https");
        }

        Ok(config)
    }

    pub fn secure(&self) -> bool {
        self.base_url.starts_with("https")
    }
}
