use crate::utils::serde::OneOrMany;
use crate::utils::{
    geoip_headers::GeoIpHeaderSource,
    ip_headers::{ClientIpHeaderSource, TrustedProxy},
};
use anyhow::{Context, Result, bail};
use config::{File, FileFormat, Value};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::num::NonZeroU16;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub base_url: String,
    listen: Option<ListenAddr>,
    port: Option<ListenAddr>,
    // don't load favicons from the duckduckgo api
    pub disable_favicons: bool,
    pub disable_ntp_check: bool,
    pub data_dir: String,

    /// Maximum lifetime of a dashboard login, without renewal.
    #[serde(with = "crate::utils::serde::human_duration")]
    pub session_duration: Duration,
    pub geoip: GeoIpConfig,
    pub duckdb: DuckdbConfig,
    pub limits: LimitsConfig,

    /// Client IP header names or provider presets.
    /// Presets: `cloudflare`, `fastly`, `fly`, `cloudfront`, and `akamai`.
    pub trusted_headers: OneOrMany<ClientIpHeaderSource>,
    pub trusted_proxies: OneOrMany<TrustedProxy>,
    pub visitor_group_rotation_hour: u8,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = if cfg!(target_family = "unix") {
            let home = std::env::var("HOME").ok().unwrap_or_else(|| "/root".to_string());
            std::env::var("XDG_DATA_HOME").map_or_else(
                |_| format!("{home}/.local/share/liwan/data"),
                |data_home| format!("{data_home}/liwan/data"),
            )
        } else {
            "./liwan-data".to_string()
        };

        Self {
            base_url: "http://localhost:9042".to_string(),
            listen: None,
            port: None,
            disable_favicons: false,
            disable_ntp_check: false,
            data_dir,
            session_duration: Duration::from_secs(24 * 60 * 60 * 14),
            geoip: Default::default(),
            duckdb: Default::default(),
            limits: Default::default(),
            trusted_headers: vec![ClientIpHeaderSource::Header("x-forwarded-for".to_string())].into(),
            trusted_proxies: vec![
                TrustedProxy::Cidr("127.0.0.1/8".parse().expect("valid default trusted proxy")),
                TrustedProxy::Cidr("::1/128".parse().expect("valid default trusted proxy")),
            ]
            .into(),
            visitor_group_rotation_hour: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeoIpConfig {
    /// GeoIP header mappings or provider presets.
    /// Presets: `akamai`, `cloudflare`, `cloudfront`, `netlify`, and `vercel`.
    pub headers: OneOrMany<GeoIpHeaderSource>,
    pub maxmind_db_path: Option<String>,
    pub maxmind_account_id: Option<MaxMindAccountId>,
    pub maxmind_license_key: Option<String>,
    pub maxmind_edition: String,
}

impl Default for GeoIpConfig {
    fn default() -> Self {
        Self {
            headers: Default::default(),
            maxmind_db_path: None,
            maxmind_account_id: None,
            maxmind_license_key: None,
            maxmind_edition: "GeoLite2-City".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DuckdbConfig {
    pub memory_limit: Option<String>,
    pub threads: Option<NonZeroU16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub report_max_concurrency: usize,
    pub report_timeout_seconds: u64,
    pub report_max_range_days: i64,
    pub report_max_dimension_results: usize,
    pub report_max_datapoints: usize,
    pub report_max_filters: usize,
    pub report_max_filter_value_bytes: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            report_max_concurrency: 8,
            report_timeout_seconds: 30,
            report_max_range_days: 3660,
            report_max_dimension_results: 1000,
            report_max_datapoints: 2000,
            report_max_filters: 20,
            report_max_filter_value_bytes: 2048,
        }
    }
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

pub static DEFAULT_CONFIG: &str = include_str!("../../../data/config.example.toml");

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
        if base_url.host_str().is_none()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || base_url.username() != ""
            || base_url.password().is_some()
        {
            bail!(
                "Invalid base URL: expected an HTTP(S) origin and optional path, without credentials, query or fragment"
            );
        }
        if config.base_url.ends_with('/') && base_url.path() != "/" {
            bail!("Invalid base URL: remove the trailing slash from the path");
        }
        if config.base_path().starts_with("//") || base_url.path().trim_end_matches('/') != config.base_path() {
            bail!("Invalid base URL: path must not contain dot segments or require URL normalization");
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
        if !(Duration::from_secs(60 * 60)..=Duration::from_secs(365 * 24 * 60 * 60)).contains(&config.session_duration)
        {
            bail!("Invalid session_duration: must be between 1 hour and 365 days");
        }
        if config.limits.report_max_concurrency == 0 || config.limits.report_max_concurrency > 10 {
            bail!("Invalid limits.report_max_concurrency: must be between 1 and 10");
        }
        if config.limits.report_timeout_seconds == 0
            || config.limits.report_max_range_days <= 0
            || config.limits.report_max_dimension_results == 0
            || config.limits.report_max_datapoints == 0
            || config.limits.report_max_filters == 0
            || config.limits.report_max_filter_value_bytes == 0
        {
            bail!("Invalid report limit: values must be greater than zero");
        }

        Ok(config)
    }

    pub fn listen_addr(&self) -> String {
        self.listen.as_ref().or(self.port.as_ref()).unwrap_or(&ListenAddr::Port(9042)).addr()
    }

    pub fn secure(&self) -> bool {
        self.base_url.split_once("://").is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
    }

    /// Returns the configured URL path prefix, or an empty string at the origin root.
    pub fn base_path(&self) -> &str {
        let url = self.base_url.split_once("://").map(|(_, rest)| rest).unwrap_or(&self.base_url);
        url.find('/').map(|index| url[index..].trim_end_matches('/')).unwrap_or("")
    }

    /// Prefixes an application path (starting with `/`) with the configured base path.
    pub fn path(&self, path: &str) -> String {
        format!("{}{path}", self.base_path())
    }

    /// Builds a public URL for an application path (starting with `/`).
    pub fn public_url(&self, path: &str) -> Result<Url> {
        let mut url = Url::parse(&self.base_url).context("Invalid base URL")?;
        url.set_path(&self.path(path));
        Ok(url)
    }
}

fn map_env_key(key: &str) -> Option<String> {
    let key = key.strip_prefix("LIWAN_")?.to_ascii_lowercase();
    if key == "geoip_headers" {
        return Some("geoip.headers".to_string());
    }
    const NESTED_PREFIXES: &[(&str, &str)] =
        &[("maxmind_", "geoip.maxmind_"), ("duckdb_", "duckdb."), ("limits_", "limits.")];

    for (prefix, mapped_prefix) in NESTED_PREFIXES {
        if let Some(rest) = key.strip_prefix(prefix) {
            return Some(format!("{mapped_prefix}{rest}"));
        }
    }

    Some(key)
}

fn parse_env_value(key: &str, value: &str) -> Value {
    if ["trusted_headers", "trusted_proxies", "geoip.headers"].contains(&key) && value.contains(',') {
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
            ("LIWAN_LIMITS_REPORT_MAX_CONCURRENCY", "6"),
            ("LIWAN_LIMITS_REPORT_TIMEOUT_SECONDS", "45"),
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
        assert_eq!(config.limits.report_max_concurrency, 6);
        assert_eq!(config.limits.report_timeout_seconds, 45);
    }

    #[test]
    fn test_empty_proxy_config_overrides_defaults() {
        let (_temp_dir, config_path) = temp_config(
            "empty-proxies.config.toml",
            r#"
                trusted_headers = []
                trusted_proxies = []
            "#,
        );

        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");

        assert!(config.trusted_headers.is_empty());
        assert!(config.trusted_proxies.is_empty());
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
            config.trusted_headers.as_ref(),
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
        assert_eq!(config.trusted_headers.as_ref(), &[ClientIpHeaderSource::Header("x-client-ip".to_string())]);
    }

    #[test]
    fn test_wildcard_trusted_proxy() {
        let env_config = Config::load(None, [("LIWAN_TRUSTED_PROXIES", "*")]).expect("failed to load config");
        assert_eq!(env_config.trusted_proxies.as_ref(), &[TrustedProxy::All]);

        let (_temp_dir, config_path) = temp_config("wildcard-proxy.config.toml", "trusted_proxies = \"*\"");
        let file_config =
            Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");
        assert_eq!(file_config.trusted_proxies.as_ref(), &[TrustedProxy::All]);
    }

    #[test]
    fn test_header_presets_and_geoip_mappings() {
        let (_temp_dir, config_path) = temp_config(
            "headers.config.toml",
            r#"
                trusted_headers = ["cloudflare", "fastly", "fly", "akamai", "X-Client-IP"]

                [geoip]
                headers = ["cloudflare", { country = "X-Country", city = "X-City" }]
            "#,
        );

        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");
        assert_eq!(
            config.trusted_headers.as_ref(),
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
            GeoIpHeaderSource::Provider(crate::utils::geoip_headers::GeoIpProvider::Cloudflare)
        );
        assert_eq!(
            config.geoip.headers[1],
            GeoIpHeaderSource::Mapping(crate::utils::geoip_headers::GeoIpHeaderMapping {
                country: "X-Country".to_string(),
                city: "X-City".to_string(),
            })
        );
    }

    #[test]
    fn test_trusted_headers_serialization() {
        let (_temp_dir, config_path) = temp_config("headers.config.toml", "trusted_headers = \"fly\"");
        let config = Config::load(Some(config_path), Vec::<(String, String)>::new()).expect("failed to load config");

        assert_eq!(config.trusted_headers.as_ref(), &[ClientIpHeaderSource::Provider(ClientIpProvider::Fly)]);
        let serialized = serde_json::to_value(config).expect("failed to serialize config");
        assert!(serialized.get("trusted_headers").is_some());
        assert!(serialized.get("client_ip_headers").is_none());
    }

    #[test]
    fn test_defaults() {
        let config = Config::load(None, Vec::<(String, String)>::new()).expect("failed to load config");
        let default = Config::default();
        assert_eq!(config.base_url, "http://localhost:9042");
        assert_eq!(config.listen_addr(), "0.0.0.0:9042");
        assert_eq!(config.data_dir, default.data_dir);
        assert_eq!(config.session_duration, Duration::from_secs(24 * 60 * 60 * 14));
        assert_eq!(config.visitor_group_rotation_hour, 4);
        assert!(config.geoip.maxmind_db_path.is_none());
        assert!(config.geoip.maxmind_account_id.is_none());
        assert!(config.geoip.maxmind_license_key.is_none());
        assert_eq!(config.geoip.maxmind_edition, "GeoLite2-City");
        assert_eq!(default.geoip.maxmind_edition, "GeoLite2-City");
        assert_eq!(config.trusted_headers.as_ref(), &[ClientIpHeaderSource::Header("x-forwarded-for".to_string())]);
        assert_eq!(config.trusted_headers, default.trusted_headers);
        assert_eq!(
            config.trusted_proxies.as_ref(),
            &[TrustedProxy::Cidr("127.0.0.1/8".parse().unwrap()), TrustedProxy::Cidr("::1/128".parse().unwrap())]
        );
        assert_eq!(config.trusted_proxies, default.trusted_proxies);
        assert_eq!(config.limits.report_max_concurrency, 8);
        assert_eq!(config.limits.report_timeout_seconds, 30);
        assert_eq!(config.limits.report_max_range_days, 3660);
        assert_eq!(config.limits.report_max_dimension_results, 1000);
        assert_eq!(config.limits.report_max_datapoints, 2000);
        assert_eq!(config.limits.report_max_filters, 20);
        assert_eq!(config.limits.report_max_filter_value_bytes, 2048);

        // a partial geoip section keeps the remaining defaults
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
        assert_eq!(config.geoip.maxmind_edition, "GeoLite2-City");
        assert_eq!(config.geoip.maxmind_db_path, Some("test2".to_string()));
        assert_eq!(config.base_url, "http://localhost:8081");
        assert_eq!(config.data_dir, "./liwan-test-data");
        assert_eq!(config.listen_addr(), "0.0.0.0:9042");
    }

    #[test]
    fn test_invalid_report_limits() {
        let error = Config::load(None, [("LIWAN_LIMITS_REPORT_TIMEOUT_SECONDS", "0")]).unwrap_err();
        assert!(error.to_string().contains("values must be greater than zero"));

        let error = Config::load(None, [("LIWAN_LIMITS_REPORT_MAX_CONCURRENCY", "11")]).unwrap_err();
        assert!(error.to_string().contains("must be between 1 and 10"));
    }

    #[test]
    fn test_session_duration() {
        let (_temp_dir, path) = temp_config("session.config.toml", "session_duration = \"2 days\"");
        assert_eq!(
            Config::load(Some(path), Vec::<(String, String)>::new()).unwrap().session_duration,
            Duration::from_secs(48 * 60 * 60)
        );
        assert_eq!(
            Config::load(None, [("LIWAN_SESSION_DURATION", "24h")]).unwrap().session_duration,
            Duration::from_secs(24 * 60 * 60)
        );
        let serialized = serde_json::to_value(Config::default()).unwrap();
        assert!(serialized["session_duration"].is_string());
        assert_eq!(
            serde_json::from_value::<Config>(serialized).unwrap().session_duration,
            Config::default().session_duration
        );
        for value in ["0", "0s", "0h", "8761h", "banana", "14", "-1d"] {
            assert!(Config::load(None, [("LIWAN_SESSION_DURATION", value)]).is_err());
        }
    }
}
