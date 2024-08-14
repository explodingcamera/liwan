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
            .merge(Env::raw().filter_map(|key| match key {
                k if !k.starts_with("LIWAN_") => None,
                k if k.starts_with("LIWAN_MAXMIND_") => Some(format!("geoip.maxmind_{}", &k[14..]).into()),
                k => Some(k[6..].as_str().to_lowercase().replace("_", ".").into()),
            }))
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

#[cfg(test)]
mod test {
    use super::*;
    use figment::Jail;

    #[test]
    fn test_config() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "liwan2.config.toml",
                r#"
                base_url = "http://localhost:8081"
                data_dir = "./liwan-test-data"
                [geoip]
                maxmind_db_path = "test2"
            "#,
            )?;

            jail.set_env("LIWAN_MAXMIND_EDITION", "test");
            jail.set_env("LIWAN_GEOIP_MAXMIND_EDITION", "test2");
            jail.set_env("GEOIP_MAXMIND_EDITION", "test3");

            jail.set_env("LIWAN_MAXMIND_LICENSE_KEY", "test");
            jail.set_env("LIWAN_MAXMIND_ACCOUNT_ID", "test");
            jail.set_env("LIWAN_MAXMIND_DB_PATH", "test");

            let config = Config::load(Some("liwan2.config.toml".into())).expect("failed to load config");

            assert_eq!(config.geoip.as_ref().unwrap().maxmind_edition, Some("test".to_string()));
            assert_eq!(config.geoip.as_ref().unwrap().maxmind_license_key, Some("test".to_string()));
            assert_eq!(config.geoip.as_ref().unwrap().maxmind_account_id, Some("test".to_string()));
            assert_eq!(config.geoip.as_ref().unwrap().maxmind_db_path, Some("test".to_string()));
            assert_eq!(config.base_url, "http://localhost:8081");
            assert_eq!(config.data_dir, "./liwan-test-data");
            assert_eq!(config.port, 8080);
            Ok(())
        });
    }

    #[test]
    fn test_no_geoip() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "liwan3.config.toml",
                r#"
                base_url = "http://localhost:8081"
                data_dir = "./liwan-test-data"
            "#,
            )?;

            let config = Config::load(Some("liwan3.config.toml".into())).expect("failed to load config");

            assert!(config.geoip.is_none());
            assert_eq!(config.base_url, "http://localhost:8081");
            assert_eq!(config.data_dir, "./liwan-test-data");
            assert_eq!(config.port, 8080);
            Ok(())
        });
    }

    #[test]
    fn test_no_config() {
        Jail::expect_with(|_jail| {
            let config = Config::load(None).expect("failed to load config");
            assert!(config.geoip.is_none());
            assert_eq!(config.base_url, "http://localhost:8080");
            assert_eq!(config.data_dir, "./liwan-data");
            assert_eq!(config.port, 8080);
            Ok(())
        });
    }
}
