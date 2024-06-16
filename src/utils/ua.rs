use lazy_static::lazy_static;
use uaparser::{Client, Parser, UserAgentParser};

lazy_static! {
    static ref PARSER: UserAgentParser = UserAgentParser::builder()
        .build_from_bytes(include_bytes!("./ua_regexes.yaml"))
        .expect("Parser creation failed");
}

pub fn parse(header: &str) -> Client {
    PARSER.parse(header)
}

pub fn is_bot(client: &Client) -> bool {
    client.device.family == "Spider"
}

static MOBILE_OS: [&str; 2] = ["iOS", "Android"]; // good enough for 99% of cases
pub fn is_mobile(client: &Client) -> bool {
    MOBILE_OS.contains(&&*client.os.family)
}
