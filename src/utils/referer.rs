use lazy_static::lazy_static;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

lazy_static! {
    static ref REFERERS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        for line in include_str!("../../data/referers.txt").lines() {
            let mut parts = line.split('=');
            let name = parts.next().unwrap();
            let fqdn = parts.next().unwrap();
            map.insert(fqdn, name);
        }
        map
    };
    static ref SPAMMERS: HashSet<&'static str> = include_str!("../../data/spammers.txt").lines().collect();
}

pub fn get_referer_name(fqdn: &str) -> Option<&'static str> {
    REFERERS.get(fqdn).copied()
}

pub fn is_spammer(fqdn: &str) -> bool {
    SPAMMERS.contains(fqdn)
}

pub fn process_referer(referer: Option<&str>) -> Result<Option<String>, ()> {
    let res = match referer.map(poem::http::Uri::from_str) {
        // valid referer are stripped to the FQDN
        Some(Ok(referer_uri)) => {
            let referer_fqn = referer_uri.host().unwrap_or_default();
            if is_spammer(referer_fqn) {
                return Err(());
            }
            Some(referer_fqn.to_owned())
        }
        // invalid referer are kept as is (e.g. when using custom referer values outside of the browser)
        Some(Err(_)) => referer.map(|r| r.to_owned()),
        None => None,
    };

    Ok(res)
}
