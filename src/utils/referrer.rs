use std::cell::LazyCell;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

thread_local! {
    pub static REFERERS: LazyCell<HashMap<String, String>> = LazyCell::new(|| {
        let mut map = HashMap::new();
        for line in include_str!("../../data/referrers.txt").lines() {
            let mut parts = line.split('=');
            let name = parts.next().unwrap();
            let fqdn = parts.next().unwrap();
            map.insert(fqdn.to_string(), name.to_string());
        }
        map
    });

    pub static REFERRER_ICONS: LazyCell<HashMap<String, String>> = LazyCell::new(|| {
        let mut map = HashMap::new();
        for line in include_str!("../../data/referrer_icons.txt").lines() {
            let mut parts = line.split('=');
            let fqdn = parts.next().unwrap();
            let icon = parts.next().unwrap();
            map.insert(fqdn.to_string(), icon.to_string());
        }
        map
    });
    pub static SPAMMERS: LazyCell<HashSet<String>> =
        LazyCell::new(|| include_str!("../../data/spammers.txt").lines().map(std::string::ToString::to_string).collect());
}

pub fn get_referer_name(fqdn: &str) -> Option<String> {
    REFERERS.with(|r| r.get(fqdn).map(std::string::ToString::to_string))
}

pub fn get_referer_icon(name: &str) -> Option<String> {
    REFERRER_ICONS.with(|r| r.get(name).map(std::string::ToString::to_string))
}

pub fn is_spammer(fqdn: &str) -> bool {
    SPAMMERS.with(|s| s.contains(fqdn))
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

        assert_eq!(
            process_referer(Some("invalid referrer")),
            Referrer::Unknown(Some("invalid referrer".to_string())),
            "Should return the original referrer if it is invalid"
        );
    }
}
