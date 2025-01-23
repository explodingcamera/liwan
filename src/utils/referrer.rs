use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::LazyLock;

pub fn get_referer_name(fqdn: &str) -> Option<String> {
    static REFERERS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        include_str!("../../data/referrers.txt")
            .lines()
            .map(|line| {
                let mut parts = line.split('=');
                let name = parts.next().expect("referrers.txt is malformed: missing key");
                let fqdn = parts.next().expect("referrers.txt is malformed: missing value");
                (fqdn.to_string(), name.to_string())
            })
            .collect()
    });

    REFERERS.get(fqdn).map(ToString::to_string)
}

pub fn get_referer_icon(name: &str) -> Option<String> {
    static REFERRER_ICONS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        include_str!("../../data/referrer_icons.txt")
            .lines()
            .map(|line| {
                let mut parts = line.split('=');
                let fqdn = parts.next().expect("referrer_icons.txt is malformed: missing key");
                let icon = parts.next().expect("referrer_icons.txt is malformed: missing value");
                (fqdn.to_string(), icon.to_string())
            })
            .collect()
    });

    REFERRER_ICONS.get(name).map(ToString::to_string)
}

pub fn is_spammer(fqdn: &str) -> bool {
    static SPAMMERS: LazyLock<HashSet<String>> =
        LazyLock::new(|| include_str!("../../data/spammers.txt").lines().map(ToString::to_string).collect());

    SPAMMERS.contains(fqdn)
}

#[derive(Debug, PartialEq)]
pub enum Referrer {
    Fqdn(String),
    Unknown(Option<String>),
    Spammer,
}

pub fn process_referer(referer: Option<&str>) -> Referrer {
    match referer.map(poem::http::Uri::from_str) {
        // valid referer are stripped to the FQDN
        Some(Ok(referer_uri)) => {
            // ignore localhost / IP addresses
            if referer_uri.host().is_some_and(|host| {
                host == "localhost" || host.ends_with(".localhost") || host.parse::<std::net::IpAddr>().is_ok()
            }) {
                return Referrer::Unknown(None);
            }

            let referer_fqn = referer_uri.host().unwrap_or_default();
            if is_spammer(referer_fqn) {
                return Referrer::Spammer;
            }
            Referrer::Fqdn(referer_fqn.to_string())
        }
        // invalid referer are kept as is (e.g. when using custom referer values outside of the browser)
        Some(Err(_)) => Referrer::Unknown(referer.map(std::string::ToString::to_string)),
        None => Referrer::Unknown(None),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process_referer() {
        assert_eq!(process_referer(None), Referrer::Unknown(None), "Should return None when no referer is provided");

        assert_eq!(
            process_referer(Some("https://example.com/path?query=string")),
            Referrer::Fqdn("example.com".to_string()),
            "Should return the FQDN for a valid referer that is not a spammer"
        );

        assert_eq!(
            process_referer(Some("https://adf.ly/path")),
            Referrer::Spammer,
            "Should return an error for a referer identified as a spammer"
        );

        assert_eq!(process_referer(Some("google.com")), Referrer::Fqdn("google.com".to_string()));
        assert_eq!(process_referer(Some("127.0.0.1")), Referrer::Unknown(None));
        assert_eq!(process_referer(Some("1.1.1.1")), Referrer::Unknown(None));
        assert_eq!(process_referer(Some("localhost")), Referrer::Unknown(None));
        assert_eq!(process_referer(Some("asdf.localhost")), Referrer::Unknown(None));

        assert_eq!(
            process_referer(Some("invalid referrer")),
            Referrer::Unknown(Some("invalid referrer".to_string())),
            "Should return the original referrer if it is invalid"
        );
    }
}
