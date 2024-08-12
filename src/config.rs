use eyre::{bail, Context, Result};
use figment::providers::{Env, Format, Toml};
use figment::Figment;
use poem::http::Uri;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

fn default_base() -> String {
    "http://localhost:8080".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_data_dir() -> String {
    "./liwan-data".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
    #[serde(default = "default_base")]
    pub(crate) base_url: String,

    #[serde(default = "default_port")]
    pub(crate) port: u16,

    #[serde(default = "default_data_dir")]
    pub(crate) data_dir: String,

    pub(crate) geoip: Option<GeoIpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeoIpConfig {
    pub(crate) maxmind_db_path: Option<String>,
    pub(crate) maxmind_account_id: Option<String>,
    pub(crate) maxmind_license_key: Option<String>,
    pub(crate) maxmind_edition: Option<String>,
}

pub(crate) static DEFAULT_CONFIG: &str = include_str!("../config.example.toml");

impl Config {
    pub(crate) fn load(path: Option<String>) -> Result<Self> {
        tracing::debug!(path = ?path, "loading config");

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

    pub(crate) fn secure(&self) -> bool {
        self.base_url.starts_with("https")
    }
}
