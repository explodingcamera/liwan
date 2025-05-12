use eyre::{Context, Result, bail};
use figment::Figment;
use figment::providers::{Env, Format, Toml};
use poem::http::Uri;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;
use std::str::FromStr;

fn default_base() -> String {
    "http://localhost:9042".to_string()
}

fn default_port() -> u16 {
    9042
}

fn default_data_dir() -> String {
    #[cfg(target_family = "unix")]
    {
        let home = std::env::var("HOME").ok().unwrap_or_else(|| "/root".to_string());
        std::env::var("XDG_DATA_HOME")
            .map_or_else(|_| format!("{home}/.local/share/liwan/data"), |data_home| format!("{data_home}/liwan/data"))
    }

    #[cfg(not(target_family = "unix"))]
    "./liwan-data".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base")]
    pub base_url: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default)]
    // don't load favicons from the duckduckgo api
    pub disable_favicons: bool,

    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    pub geoip: Option<GeoIpConfig>,

    pub duckdb: Option<DuckdbConfig>,
}

#[must_use]
pub fn default_maxmind_edition() -> Option<String> {
    Some("GeoLite2-City".to_string())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base(),
            port: default_port(),
            data_dir: default_data_dir(),
            geoip: None,
            duckdb: None,
            disable_favicons: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpConfig {
    pub maxmind_db_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_from_number")]
    pub maxmind_account_id: Option<String>,
    pub maxmind_license_key: Option<String>,
    #[serde(default = "default_maxmind_edition")]
    pub maxmind_edition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckdbConfig {
    pub memory_limit: Option<String>,
    pub threads: Option<NonZeroU16>,
}

pub fn deserialize_string_from_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(i64),
        Float(f64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(Some(s)),
        StringOrNumber::Number(i) => Ok(Some(i.to_string())),
        StringOrNumber::Float(f) => Ok(Some(f.to_string())),
    }
}

pub static DEFAULT_CONFIG: &str = include_str!("../data/config.example.toml");

impl Config {
    pub fn load(path: Option<String>) -> Result<Self> {
        tracing::debug!(path = ?path, "loading config");

        let path = path.or_else(|| std::env::var("LIWAN_CONFIG").ok());

        let mut config = Figment::new()
            .merge(Toml::file("liwan.config.toml"))
            .merge(Toml::file(path.unwrap_or("liwan.config.toml".to_string())));

        #[cfg(target_family = "unix")]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));

            config = config
                .join(Toml::file(format!("{config_dir}/liwan/config.toml")))
                .join(Toml::file(format!("{config_dir}/liwan/liwan.config.toml")))
                .join(Toml::file(format!("{config_dir}/liwan.config.toml")));
        }

        let config: Self = config
            .merge(Env::raw().filter_map(|key| match key {
                k if !k.starts_with("LIWAN_") => None,
                k if k.starts_with("LIWAN_MAXMIND_") => Some(format!("geoip.maxmind_{}", &k[14..]).into()),
                k if k.starts_with("LIWAN_DUCKDB_") => Some(format!("duckdb.{}", &k[13..]).into()),
                k => Some(k[6..].as_str().to_lowercase().into()),
            }))
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
            jail.set_env("LIWAN_DUCKDB_MEMORY_LIMIT", "2GB");
            jail.set_env("LIWAN_DUCKDB_THREADS", 4);

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
            assert_eq!(config.port, 9042);
            assert_eq!(config.duckdb.as_ref().unwrap().memory_limit, Some("2GB".to_string()));
            assert_eq!(config.duckdb.as_ref().unwrap().threads, Some(NonZeroU16::new(4).unwrap()));
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
            assert_eq!(config.port, 9042);
            Ok(())
        });
    }

    #[test]
    fn test_default_geoip() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "liwan3.config.toml",
                r#"
                base_url = "http://localhost:8081"
                data_dir = "./liwan-test-data"
                [geoip]
                maxmind_db_path = "test2"
            "#,
            )?;

            let config = Config::load(Some("liwan3.config.toml".into())).expect("failed to load config");
            assert_eq!(config.geoip.clone().unwrap().maxmind_edition, Some("GeoLite2-City".to_string()));
            assert_eq!(config.geoip.unwrap().maxmind_db_path, Some("test2".to_string()));
            assert_eq!(config.base_url, "http://localhost:8081");
            assert_eq!(config.data_dir, "./liwan-test-data");
            Ok(())
        });
    }

    #[test]
    fn test_env() {
        Jail::expect_with(|jail| {
            jail.set_env("LIWAN_DATA_DIR", "/data");
            jail.set_env("LIWAN_BASE_URL", "https://example.com");
            jail.set_env("LIWAN_MAXMIND_ACCOUNT_ID", 123);
            let config = Config::load(None).expect("failed to load config");
            assert_eq!(config.data_dir, "/data");
            assert_eq!(config.base_url, "https://example.com");
            assert_eq!(config.geoip.unwrap().maxmind_account_id, Some("123".to_string()));
            Ok(())
        });
    }

    #[test]
    fn test_no_config() {
        Jail::expect_with(|_jail| {
            let config = Config::load(None).expect("failed to load config");
            assert!(config.geoip.is_none());
            assert_eq!(config.base_url, "http://localhost:9042");
            assert_eq!(config.port, 9042);
            Ok(())
        });
    }
}
