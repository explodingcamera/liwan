use std::cell::LazyCell;
use std::collections::HashMap;

thread_local! {
    pub static COUNTRIES: LazyCell<HashMap<String, String>> = LazyCell::new(|| {
        let mut map = HashMap::new();
        for line in include_str!("../../data/countries.txt").lines() {
            let mut parts = line.split('=');
            let name = parts.next().unwrap();
            let fqdn = parts.next().unwrap();
            map.insert(fqdn.to_string(), name.to_string());
        }
        map
    });
}

pub fn get_country_name(iso_2_code: &str) -> Option<String> {
    COUNTRIES.with(|r| r.get(iso_2_code).map(std::string::ToString::to_string))
}
