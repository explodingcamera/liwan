use quick_cache::sync::Cache;
use std::sync::LazyLock;
use uaparser::{Parser, UserAgentParser};

#[derive(Clone, Debug, Default)]
pub struct UserAgent {
    pub device_family: String,
    pub os_family: String,
    pub ua_family: String,
}

static PARSER: LazyLock<UserAgentParser> = LazyLock::new(|| {
    UserAgentParser::builder()
        .with_unicode_support(false)
        .build_from_bytes(include_bytes!("../../data/ua_regexes.yaml"))
        .expect("Parser creation failed")
});

static UAP_CACHE: LazyLock<Cache<String, UserAgent>> = LazyLock::new(|| Cache::new(1024));

pub fn parse(header: &str) -> UserAgent {
    if let Some(client) = UAP_CACHE.get(header) {
        return client;
    }

    let client = PARSER.parse(header);
    let uap = UserAgent {
        device_family: client.device.family.to_string(),
        os_family: client.os.family.replace("Mac OS X", "macOS").replace("Other", "Unknown").into(),
        ua_family: client.user_agent.family.to_string(),
    };

    UAP_CACHE.insert(header.to_string(), uap.clone());
    uap
}

impl UserAgent {
    pub fn from_header(header: &str) -> Self {
        if let Some(client) = UAP_CACHE.get(header) {
            return client.clone();
        }

        let client = PARSER.parse(header);
        let uap = UserAgent {
            device_family: client.device.family.to_string(),
            os_family: client.os.family.replace("Mac OS X", "macOS").replace("Other", "Unknown").into(),
            ua_family: client.user_agent.family.to_string(),
        };

        UAP_CACHE.insert(header.to_string(), uap.clone());
        uap
    }

    pub fn is_bot(&self) -> bool {
        self.device_family == "Spider" || self.ua_family == "HeadlessChrome"
    }

    pub fn is_mobile(&self) -> bool {
        ["iOS", "Android"].contains(&self.os_family.as_str()) // good enough for 99% of cases
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ua_parser() {
        let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1";
        let client = parse(user_agent);
        assert_eq!(client.os_family, "iOS", "Expected OS family to be iOS");
        assert_eq!(client.device_family, "iPhone", "Expected device family to be iPhone");
        assert!(client.is_mobile(), "Expected device to be mobile");
        assert!(!client.is_bot(), "Expected device to not be a bot");
    }
}
