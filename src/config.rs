use crate::utils::ip_headers::{TrustedHeader, TrustedProxy, deserialize_trusted_headers, deserialize_trusted_proxies};
use anyhow::{Context, Result, bail};
use figment::Figment;
use figment::providers::{Env, Format, Toml};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::num::NonZeroU16;
use std::str::FromStr;
use url::Url;

fn default_base() -> String {
    "http://localhost:9042".to_string()
}

fn default_port() -> u16 {
    9042
}

fn default_listen() -> ListenAddr {
    ListenAddr::Port(default_port())
}

#[must_use]
pub fn default_maxmind_edition() -> String {
    "GeoLite2-City".to_string()
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

fn default_trusted_headers() -> Vec<TrustedHeader> {
    TrustedHeader::all().to_vec()
}

fn default_use_forward_headers() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_base")]
    pub base_url: String,

    #[serde(default)]
    listen: Option<ListenAddr>,

    #[serde(default)]
    port: Option<ListenAddr>,

    #[serde(default)]
    // don't load favicons from the duckduckgo api
    pub disable_favicons: bool,

    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    #[serde(default)]
    pub geoip: GeoIpConfig,

    #[serde(default)]
    pub duckdb: DuckdbConfig,

    #[serde(default = "default_trusted_headers", deserialize_with = "deserialize_trusted_headers")]
    pub trusted_headers: Vec<TrustedHeader>,

    #[serde(default, deserialize_with = "deserialize_trusted_proxies")]
    pub trusted_proxies: Vec<TrustedProxy>,

    #[serde(default = "default_use_forward_headers")]
    pub use_forward_headers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ListenAddr {
    Port(u16),
    Addr(String),
}

impl ListenAddr {
    pub fn addr(&self) -> String {
        match self {
            ListenAddr::Port(port) => SocketAddr::from(([0, 0, 0, 0], *port)).to_string(),
            ListenAddr::Addr(addr) => addr.clone(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base(),
            data_dir: default_data_dir(),
            geoip: Default::default(),
            duckdb: Default::default(),
            disable_favicons: false,
            listen: None,
            port: None,
            trusted_headers: default_trusted_headers(),
            trusted_proxies: Vec::new(),
            use_forward_headers: default_use_forward_headers(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoIpConfig {
    #[serde(default)]
    pub maxmind_db_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_from_number")]
    pub maxmind_account_id: Option<String>,
    #[serde(default)]
    pub maxmind_license_key: Option<String>,
    #[serde(default = "default_maxmind_edition")]
    pub maxmind_edition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DuckdbConfig {
    #[serde(default)]
    pub memory_limit: Option<String>,
    #[serde(default)]
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

        let url: Url = Url::from_str(&config.base_url).context("Invalid base URL")?;

        if !["http", "https"].contains(&url.scheme()) {
            bail!("Invalid base URL: protocol must be either http or https");
        }

        if url.scheme() != "https" {
            tracing::warn!("Base URL is not using HTTPS");
        }

        if config.listen.is_some() && config.port.is_some() {
            tracing::warn!(
                "Both `listen` and `port` configuration options are set. The `listen` option will take precedence over `port`."
            );
        }

        Ok(config)
    }

    pub fn listen_addr(&self) -> String {
        self.listen.as_ref().or(self.port.as_ref()).unwrap_or(&default_listen()).addr()
    }

    pub fn secure(&self) -> bool {
        self.base_url.starts_with("https")
    }
}

#[cfg(test)]
#[allow(clippy::result_large_err)]
mod test {
    use super::*;
    use crate::utils::ip_headers::{TrustedHeader, TrustedProxy};
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

            assert_eq!(config.geoip.maxmind_edition, "test".to_string());
            assert_eq!(config.geoip.maxmind_license_key, Some("test".to_string()));
            assert_eq!(config.geoip.maxmind_account_id, Some("test".to_string()));
            assert_eq!(config.geoip.maxmind_db_path, Some("test".to_string()));
            assert_eq!(config.base_url, "http://localhost:8081");
            assert_eq!(config.data_dir, "./liwan-test-data");
            assert_eq!(config.listen_addr(), "0.0.0.0:9042");
            assert_eq!(config.duckdb.memory_limit, Some("2GB".to_string()));
            assert_eq!(config.duckdb.threads, Some(NonZeroU16::new(4).unwrap()));
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

            assert!(config.geoip.maxmind_db_path.is_none());
            assert!(config.geoip.maxmind_account_id.is_none());
            assert!(config.geoip.maxmind_license_key.is_none());
            assert_eq!(config.base_url, "http://localhost:8081");
            assert_eq!(config.data_dir, "./liwan-test-data");
            assert_eq!(config.listen_addr(), "0.0.0.0:9042");
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
            assert_eq!(config.geoip.maxmind_edition, default_maxmind_edition());
            assert_eq!(config.geoip.maxmind_db_path, Some("test2".to_string()));
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
            jail.set_env("LIWAN_TRUSTED_HEADERS", "X_Forwarded_For,Forwarded");
            jail.set_env("LIWAN_TRUSTED_PROXIES", "127.0.0.1,10.0.0.0/8");
            let config = Config::load(None).expect("failed to load config");
            assert_eq!(config.data_dir, "/data");
            assert_eq!(config.base_url, "https://example.com");
            assert_eq!(config.geoip.maxmind_account_id, Some("123".to_string()));
            assert_eq!(config.trusted_headers, vec![TrustedHeader::XForwardedFor, TrustedHeader::Forwarded]);
            assert_eq!(
                config.trusted_proxies,
                vec![TrustedProxy::Ip("127.0.0.1".parse().unwrap()), TrustedProxy::Cidr("10.0.0.0/8".parse().unwrap())]
            );
            assert!(config.use_forward_headers);
            Ok(())
        });
    }

    #[test]
    fn test_env_custom_trusted_header() {
        Jail::expect_with(|jail| {
            jail.set_env("LIWAN_TRUSTED_HEADERS", "X_CLIENT_IP");
            let config = Config::load(None).expect("failed to load config");
            assert_eq!(config.trusted_headers, vec![TrustedHeader::Other("x-client-ip".to_string())]);
            Ok(())
        });
    }

    #[test]
    fn test_no_config() {
        Jail::expect_with(|_jail| {
            let config = Config::load(None).expect("failed to load config");
            assert!(config.geoip.maxmind_db_path.is_none());
            assert!(config.geoip.maxmind_account_id.is_none());
            assert!(config.geoip.maxmind_license_key.is_none());
            assert_eq!(config.base_url, "http://localhost:9042");
            assert_eq!(config.listen_addr(), "0.0.0.0:9042");
            Ok(())
        });
    }
}
