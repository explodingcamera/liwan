use std::sync::LazyLock;

use ahash::{HashMap, HashSet};

static REFERRERS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
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

static SPAMMERS: LazyLock<HashSet<String>> =
    LazyLock::new(|| include_str!("../../data/spammers.txt").lines().map(ToString::to_string).collect());

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

pub fn get_referer_name(fqdn: &str) -> Option<String> {
    REFERRERS.get(fqdn).map(ToString::to_string)
}

pub fn get_referer_icon(name: &str) -> Option<String> {
    REFERRER_ICONS.get(name).map(ToString::to_string)
}

pub fn is_spammer(fqdn: &str) -> bool {
    SPAMMERS.contains(fqdn)
}

#[derive(Debug, PartialEq)]
pub enum Referrer {
    Fqdn(String),
    Unknown(Option<String>),
    Local,
    Spammer,
}

pub fn process_referer(referer: Option<&str>) -> Referrer {
    if referer.is_some_and(|referer| {
        referer.parse::<std::net::IpAddr>().is_ok() || referer == "localhost" || referer.ends_with(".localhost")
    }) {
        return Referrer::Local;
    }

    let original = referer;
    let referer = original.map(|referer| {
        // if the referer doesn't start with a scheme, add "http://" to make it parseable as a URL
        if referer.contains("://") { referer.to_string() } else { format!("http://{}", referer) }
    });

    match referer.as_deref().map(url::Url::parse) {
        // valid referer are stripped to the FQDN
        Some(Ok(referer_uri)) => {
            // ignore localhost / IP addresses
            if referer_uri.host_str().is_some_and(|host| {
                host == "localhost" || host.ends_with(".localhost") || host.parse::<std::net::IpAddr>().is_ok()
            }) {
                return Referrer::Local;
            }

            let referer_fqn = referer_uri.host_str().unwrap_or_default();
            if is_spammer(referer_fqn) {
                return Referrer::Spammer;
            }
            Referrer::Fqdn(referer_fqn.to_string())
        }
        // invalid referer are kept as is (e.g. when using custom referer values outside of the browser)
        Some(Err(_)) => Referrer::Unknown(original.map(ToString::to_string)),
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
        assert_eq!(process_referer(Some("127.0.0.1")), Referrer::Local);
        assert_eq!(process_referer(Some("1.1.1.1")), Referrer::Local);
        assert_eq!(process_referer(Some("localhost")), Referrer::Local);
        assert_eq!(process_referer(Some("asdf.localhost")), Referrer::Local);

        assert_eq!(
            process_referer(Some("invalid referrer")),
            Referrer::Unknown(Some("invalid referrer".to_string())),
            "Should return the original referrer if it is invalid"
        );
    }
}
