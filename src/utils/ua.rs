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
