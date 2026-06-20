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

    ua_parser::Extractor::try_from(regexes).expect("valid data")
});

static UAP_CACHE: LazyLock<Cache<String, UserAgent>> = LazyLock::new(|| Cache::new(1024));
static CRAWLER_TOKENS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    include_str!("../../data/crawlers.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
});

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

pub fn is_crawler_header(header: &str) -> bool {
    let header = header.to_ascii_lowercase();
    CRAWLER_TOKENS.iter().any(|crawler| header.contains(crawler))
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

    #[test]
    fn crawler_header_matches_long_browser_like_user_agents() {
        let applebot = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1 Applebot/0.1";
        let bytespider = "Mozilla/5.0 (Linux; Android 10; Pixel 4 XL) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36 Bytespider";
        let google_other = "Mozilla/5.0 (Linux; Android 6.0.1; Nexus 5X Build/MMB29P) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36 (compatible; GoogleOther)";
        let yisou = "Mozilla/5.0 (Linux; Android 11; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.45 Mobile Safari/537.36 YisouSpider";

        assert!(is_crawler_header(applebot));
        assert!(is_crawler_header(bytespider));
        assert!(is_crawler_header(google_other));
        assert!(is_crawler_header(yisou));
    }

    #[test]
    fn crawler_header_matching_is_case_insensitive() {
        assert!(is_crawler_header("Mozilla/5.0 APPLEBOT/0.1"));
        assert!(is_crawler_header("Mozilla/5.0 ByteSpider"));
    }

    #[test]
    fn crawler_header_ignores_normal_browsers() {
        let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
        assert!(!is_crawler_header(user_agent));
    }
}
