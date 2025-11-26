use quick_cache::sync::Cache;
use std::{io::Cursor, sync::LazyLock};
use ua_parser::Extractor;

#[derive(Clone, Debug, Default)]
pub struct UserAgent {
    pub device_family: Option<String>,
    pub os_family: Option<String>,
    pub ua_family: Option<String>,
}

static PARSER: LazyLock<Extractor<'static>> = LazyLock::new(|| {
    let data = zstd::decode_all(Cursor::new(include_bytes!("../../data/ua_regexes.json.zstd"))).expect("valid data");
    let regexes: ua_parser::Regexes = serde_json::from_slice(&data).expect("valid data");
    let extractor = ua_parser::Extractor::try_from(regexes).expect("valid data");
    extractor
});

static UAP_CACHE: LazyLock<Cache<String, UserAgent>> = LazyLock::new(|| Cache::new(1024));

pub fn parse(header: &str) -> UserAgent {
    if let Some(client) = UAP_CACHE.get(header) {
        return client;
    };
    let (ua, os, device) = PARSER.extract(header);
    let uap = UserAgent {
        device_family: device.map(|d| d.device.to_string()),
        os_family: os.map(|os| os.os.replace("Mac OS X", "macOS").replace("Other", "Unknown")),
        ua_family: ua.map(|ua| ua.family.to_string()),
    };

    UAP_CACHE.insert(header.to_string(), uap.clone());
    uap
}

impl UserAgent {
    pub fn from_header(header: &str) -> Self {
        if let Some(client) = UAP_CACHE.get(header) {
            return client.clone();
        }
        parse(header)
    }

    pub fn is_bot(&self) -> bool {
        self.device_family == Some("Spider".into()) || self.ua_family == Some("HeadlessChrome".into())
    }

    pub fn is_mobile(&self) -> bool {
        [Some("iOS"), Some("Android")].contains(&self.os_family.as_deref()) // good enough for 99% of cases
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ua_parser() {
        let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1";
        let client = parse(user_agent);
        assert_eq!(client.os_family, Some("iOS".into()), "Expected OS family to be iOS");
        assert_eq!(client.device_family, Some("iPhone".into()), "Expected device family to be iPhone");
        assert!(client.is_mobile(), "Expected device to be mobile");
        assert!(!client.is_bot(), "Expected device to not be a bot");
    }
}
