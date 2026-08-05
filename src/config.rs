use crate::utils::ip_headers::{ClientIpHeaderSource, GeoIpHeaderSource, TrustedProxy};
use crate::utils::serde::OneOrMany;
use anyhow::{Context, Result, bail};
use config::{File, FileFormat, Value};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::num::NonZeroU16;
use std::str::FromStr;
use url::Url;

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

    /// Connection url for the app database, e.g. `sqlite://liwan-app.sqlite?mode=rwc`.
    /// Defaults to `liwan-app.sqlite` inside the data directory.
    #[serde(default)]
    pub database: Option<String>,

    #[serde(default)]
    pub geoip: GeoIpConfig,

    #[serde(default)]
    pub duckdb: DuckdbConfig,

    /// Client IP header names or provider presets.
    /// Presets: `cloudflare`, `fastly`, `fly`, `cloudfront`, and `akamai`.
    #[serde(default, alias = "trusted_headers")]
    pub client_ip_headers: OneOrMany<ClientIpHeaderSource>,

    #[serde(default)]
    pub trusted_proxies: OneOrMany<TrustedProxy>,

    #[serde(default = "default_visitor_group_rotation_hour")]
    pub visitor_group_rotation_hour: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base(),
            data_dir: default_data_dir(),
            database: None,
            geoip: Default::default(),
            duckdb: Default::default(),
            disable_favicons: false,
            listen: None,
            port: None,
            client_ip_headers: OneOrMany::default(),
            trusted_proxies: OneOrMany::default(),
            visitor_group_rotation_hour: default_visitor_group_rotation_hour(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoIpConfig {
    /// GeoIP header mappings or provider presets.
    /// Presets: `akamai`, `cloudflare`, `cloudfront`, `netlify`, and `vercel`.
    #[serde(default)]
    pub headers: OneOrMany<GeoIpHeaderSource>,
    #[serde(default)]
    pub maxmind_db_path: Option<String>,
    #[serde(default)]
    pub maxmind_account_id: Option<MaxMindAccountId>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum MaxMindAccountId {
    String(String),
    Number(u64),
}

impl std::fmt::Display for MaxMindAccountId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(value) => formatter.write_str(value),
            Self::Number(value) => value.fmt(formatter),
        }
    }
}

fn default_base() -> String {
    "http://localhost:9042".to_string()
}

fn default_port() -> u16 {
    9042
}

fn default_listen() -> ListenAddr {
    ListenAddr::Port(default_port())
}

fn default_maxmind_edition() -> String {
    "GeoLite2-City".to_string()
}

fn default_data_dir() -> String {
    if cfg!(target_family = "unix") {
        let home = std::env::var("HOME").ok().unwrap_or_else(|| "/root".to_string());
        std::env::var("XDG_DATA_HOME")
            .map_or_else(|_| format!("{home}/.local/share/liwan/data"), |data_home| format!("{data_home}/liwan/data"))
    } else {
        "./liwan-data".to_string()
    }
}

fn default_visitor_group_rotation_hour() -> u8 {
    4
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

pub static DEFAULT_CONFIG: &str = include_str!("../data/config.example.toml");

impl Config {
    pub fn load<I, K, V>(path: Option<String>, env_vars: I) -> Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let path = path.or_else(|| std::env::var("LIWAN_CONFIG").ok());
        let mut builder = config::Config::builder();

        #[cfg(all(not(test), target_family = "unix"))]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
            builder = builder
                .add_source(File::new(&format!("{config}/liwan/config.toml"), FileFormat::Toml).required(false))
                .add_source(File::new(&format!("{config}/liwan/liwan.config.toml"), FileFormat::Toml).required(false))
                .add_source(File::new(&format!("{config}/liwan.config.toml"), FileFormat::Toml).required(false));

            builder = builder.add_source(File::new("liwan.config.toml", FileFormat::Toml).required(false));
        }

        if let Some(path) = path {
            builder = builder.add_source(File::new(&path, FileFormat::Toml).required(false));
        }

        for (key, value) in env_vars {
            if let Some(mapped_key) = map_env_key(key.as_ref()) {
                builder = builder.set_override(&mapped_key, parse_env_value(&mapped_key, value.as_ref()))?;
            };
        }

        let config: Self = builder.build()?.try_deserialize()?;

        let base_url: Url = Url::from_str(&config.base_url).context("Invalid base URL")?;
        if !["http", "https"].contains(&base_url.scheme()) {
            bail!("Invalid base URL: protocol must be either http or https");
        }
        if base_url.scheme() != "https" {
            tracing::warn!("Base URL is not using HTTPS");
        }
        if config.listen.is_some() && config.port.is_some() {
            tracing::warn!(
                "Both `listen` and `port` configuration options are set. The `listen` option will take precedence over `port`."
            );
        }
        if config.visitor_group_rotation_hour > 23 {
            bail!("Invalid visitor_group_rotation_hour: must be between 0 and 23");
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

fn map_env_key(key: &str) -> Option<String> {
    // the database connection url may also be provided without the `LIWAN_` prefix,
    // since `DATABASE` is a conventional name
    if key == "DATABASE" {
        return Some("database".to_string());
    }

    let key = key.strip_prefix("LIWAN_")?.to_ascii_lowercase();
    if key == "geoip_headers" {
        return Some("geoip.headers".to_string());
    }
    const NESTED_PREFIXES: &[(&str, &str)] = &[("maxmind_", "geoip.maxmind_"), ("duckdb_", "duckdb.")];

    for (prefix, mapped_prefix) in NESTED_PREFIXES {
        if let Some(rest) = key.strip_prefix(prefix) {
            return Some(format!("{mapped_prefix}{rest}"));
        }
    }

    Some(key)
}

fn parse_env_value(key: &str, value: &str) -> Value {
    if ["client_ip_headers", "trusted_headers", "trusted_proxies", "geoip.headers"].contains(&key)
        && value.contains(',')
    {
        return Value::from(value.split(',').map(str::trim).collect::<Vec<_>>());
    }

    value
        .parse::<bool>()
        .ok()
        .map(Value::from)
        .or_else(|| value.parse::<i64>().ok().map(Value::from))
        .or_else(|| value.parse::<f64>().ok().map(Value::from))
        .unwrap_or_else(|| Value::from(value.to_string()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::ip_headers::{ClientIpHeaderSource, ClientIpProvider, TrustedProxy};
    use tempfile::TempDir;

    fn temp_config(name: &str, content: &str) -> (TempDir, String) {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = temp_dir.path().join(name);
        std::fs::write(&path, content).expect("failed to create config file");
        (temp_dir, path.to_string_lossy().into_owned())
    }

    #[test]
    fn test_config() {
        let (_temp_dir, config_path) = temp_config(
            "liwan2.config.toml",
            r#"
                base_url = "http://localhost:8081"
                data_dir = "./liwan-test-data"
                [geoip]
                maxmind_db_path = "test2"
            "#,
        );

        let env = vec![
            ("LIWAN_MAXMIND_EDITION", "test"),
            ("LIWAN_GEOIP_MAXMIND_EDITION", "test2"),
            ("GEOIP_MAXMIND_EDITION", "test3"),
            ("LIWAN_DUCKDB_MEMORY_LIMIT", "2GB"),
            ("LIWAN_DUCKDB_THREADS", "4"),
            ("LIWAN_MAXMIND_LICENSE_KEY", "test"),
            ("LIWAN_MAXMIND_ACCOUNT_ID", "test"),
            ("LIWAN_MAXMIND_DB_PATH", "test"),
        ];

        let config = Config::load(Some(config_path), env).expect("failed to load config");

        assert_eq!(config.geoip.maxmind_edition, "test".to_string());
        assert_eq!(config.geoip.maxmind_license_key, Some("test".to_string()));
        assert_eq!(config.geoip.maxmind_account_id, Some(MaxMindAccountId::String("test".to_string())));
        assert_eq!(config.geoip.maxmind_db_path, Some("test".to_string()));
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.data_dir, "./liwan-test-data");
        assert_eq!(config.listen_addr(), "0.0.0.0:9042");
        assert_eq!(config.duckdb.memory_limit, Some("2GB".to_string()));
        assert_eq!(config.duckdb.threads, Some(NonZeroU16::new(4).unwrap()));
    }

    #[test]
    fn test_no_geoip() {
        let (_temp_dir, config_path) = temp_config(
            "liwan3.config.toml",
            r#"
                base_url = "http://localhost:8081"
                data_dir = "./liwan-test-data"
            "#,
        );

        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");

        assert!(config.geoip.maxmind_db_path.is_none());
        assert!(config.geoip.maxmind_account_id.is_none());
        assert!(config.geoip.maxmind_license_key.is_none());
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.data_dir, "./liwan-test-data");
        assert_eq!(config.listen_addr(), "0.0.0.0:9042");
    }

    #[test]
    fn test_default_geoip() {
        let (_temp_dir, config_path) = temp_config(
            "liwan3.config.toml",
            r#"
                base_url = "http://localhost:8081"
                data_dir = "./liwan-test-data"
                [geoip]
                maxmind_db_path = "test2"
            "#,
        );

        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");
        assert_eq!(config.geoip.maxmind_edition, default_maxmind_edition());
        assert_eq!(config.geoip.maxmind_db_path, Some("test2".to_string()));
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.data_dir, "./liwan-test-data");
    }

    #[test]
    fn test_env() {
        let env = vec![
            ("LIWAN_DATA_DIR", "/data"),
            ("LIWAN_BASE_URL", "https://example.com"),
            ("LIWAN_MAXMIND_ACCOUNT_ID", "123"),
            ("LIWAN_TRUSTED_HEADERS", "X_Forwarded_For,Forwarded"),
            ("LIWAN_TRUSTED_PROXIES", "127.0.0.1,10.0.0.0/8"),
        ];

        let config = Config::load(None, env).expect("failed to load config");
        assert_eq!(config.data_dir, "/data");
        assert_eq!(config.base_url, "https://example.com");
        assert_eq!(config.geoip.maxmind_account_id, Some(MaxMindAccountId::Number(123)));
        assert_eq!(
            config.client_ip_headers.as_ref(),
            &[
                ClientIpHeaderSource::Header("x-forwarded-for".to_string()),
                ClientIpHeaderSource::Header("forwarded".to_string())
            ]
        );
        assert_eq!(
            config.trusted_proxies.as_ref(),
            &[TrustedProxy::Ip("127.0.0.1".parse().unwrap()), TrustedProxy::Cidr("10.0.0.0/8".parse().unwrap())]
        );
    }

    #[test]
    fn test_env_custom_trusted_header() {
        let config = Config::load(None, vec![("LIWAN_TRUSTED_HEADERS", "X_CLIENT_IP")]).expect("failed to load config");
        assert_eq!(config.client_ip_headers.as_ref(), &[ClientIpHeaderSource::Header("x-client-ip".to_string())]);
    }

    #[test]
    fn test_header_presets_and_geoip_mappings() {
        let (_temp_dir, config_path) = temp_config(
            "headers.config.toml",
            r#"
                client_ip_headers = ["cloudflare", "fastly", "fly", "akamai", "X-Client-IP"]

                [geoip]
                headers = ["cloudflare", { country = "X-Country", city = "X-City" }]
            "#,
        );

        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");
        assert_eq!(
            config.client_ip_headers.as_ref(),
            &[
                ClientIpHeaderSource::Provider(ClientIpProvider::Cloudflare),
                ClientIpHeaderSource::Provider(ClientIpProvider::Fastly),
                ClientIpHeaderSource::Provider(ClientIpProvider::Fly),
                ClientIpHeaderSource::Provider(ClientIpProvider::Akamai),
                ClientIpHeaderSource::Header("x-client-ip".to_string())
            ]
        );
        assert_eq!(config.geoip.headers.len(), 2);
        assert_eq!(
            config.geoip.headers[0],
            GeoIpHeaderSource::Provider(crate::utils::ip_headers::GeoIpProvider::Cloudflare)
        );
        assert_eq!(
            config.geoip.headers[1],
            GeoIpHeaderSource::Mapping(crate::utils::ip_headers::GeoIpHeaderMapping {
                country: "X-Country".to_string(),
                city: "X-City".to_string(),
            })
        );
    }

    #[test]
    fn test_legacy_trusted_headers_alias() {
        let (_temp_dir, config_path) = temp_config("legacy.config.toml", "trusted_headers = \"fly\"");
        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");

        assert_eq!(config.client_ip_headers.as_ref(), &[ClientIpHeaderSource::Provider(ClientIpProvider::Fly)]);
        let serialized = serde_json::to_value(config).expect("failed to serialize config");
        assert!(serialized.get("client_ip_headers").is_some());
        assert!(serialized.get("trusted_headers").is_none());
    }

    #[test]
    fn test_no_config() {
        let config = Config::load(None, Vec::<(String, String)>::new()).expect("failed to load config");
        assert!(config.geoip.maxmind_db_path.is_none());
        assert!(config.geoip.maxmind_account_id.is_none());
        assert!(config.geoip.maxmind_license_key.is_none());
        assert_eq!(config.base_url, "http://localhost:9042");
        assert_eq!(config.listen_addr(), "0.0.0.0:9042");
        assert!(config.client_ip_headers.is_empty());
    }
}
