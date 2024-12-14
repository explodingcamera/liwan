use std::collections::HashMap;
use std::sync::LazyLock;

pub fn get_country_name(iso_2_code: &str) -> Option<String> {
    static COUNTRIES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        include_str!("../../data/countries.txt")
            .lines()
            .map(|line| {
                let mut parts = line.split('=');
                let name = parts.next().expect("countries.txt is malformed").to_string();
                let fqdn = parts.next().expect("countries.txt is malformed").to_string();
                (fqdn, name)
            })
            .collect()
    });

    COUNTRIES.get(iso_2_code).map(ToString::to_string)
}
