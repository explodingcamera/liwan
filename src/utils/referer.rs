use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

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
