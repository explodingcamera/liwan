use std::cell::LazyCell;
use uaparser::{Client, Parser, UserAgentParser};

thread_local! {
    static PARSER: LazyCell<UserAgentParser> = LazyCell::new(|| {
        UserAgentParser::builder()
            .build_from_bytes(include_bytes!("../../data/ua_regexes.yaml"))
            .expect("Parser creation failed")
    });
}

pub(crate) fn parse(header: &str) -> Client {
    let mut client = PARSER.with(|p| p.parse(header));
    client.os.family = client.os.family.replace("Mac OS X", "macOS").replace("Other", "Unknown").into();
    client
}

pub(crate) fn is_bot(client: &Client) -> bool {
    client.device.family == "Spider"
}

static MOBILE_OS: [&str; 2] = ["iOS", "Android"]; // good enough for 99% of cases
pub(crate) fn is_mobile(client: &Client) -> bool {
    MOBILE_OS.contains(&&*client.os.family)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ua_parser() {
        let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1";
        let client = parse(user_agent);
        assert_eq!(client.os.family, "iOS", "Expected OS family to be iOS");
        assert_eq!(client.device.family, "iPhone", "Expected device family to be iPhone");
        assert!(is_mobile(&client), "Expected device to be mobile");
        assert!(!is_bot(&client), "Expected device to not be a bot");
    }
}
