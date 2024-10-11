#![allow(dead_code)]
use rand::Rng;
use time::OffsetDateTime;

use crate::app::models::Event;

const PATHS: &[&str] = &["/", "/about", "/contact", "/pricing", "/blog", "/login", "/signup"];
const REFERRERS: &[&str] = &["", "google.com", "twitter.com", "liwan.dev", "example.com", "henrygressmann.de"];
const PLATFORMS: &[&str] = &["", "Windows", "macOS", "Linux", "Android", "iOS"];
const BROWSERS: &[&str] = &["", "Chrome", "Firefox", "Safari", "Edge", "Opera"];
const CITIES: &[(&str, &str)] = &[
    ("", ""),
    ("Paris", "FR"),
    ("London", "GB"),
    ("Berlin", "DE"),
    ("Frankfurt", "DE"),
    ("New York", "US"),
    ("San Francisco", "US"),
    ("Tokyo", "JP"),
    ("Sydney", "AU"),
];

pub fn random_events(
    time_range: (OffsetDateTime, OffsetDateTime),
    entity_id: &str,
    fqdn: &str,
    count: usize,
) -> impl Iterator<Item = Event> {
    let mut rng = rand::thread_rng();
    let mut generated = 0;
    let entity_id = entity_id.to_string();
    let fqdn = fqdn.to_string();
    let visitor_ids: Vec<String> = (0..count / 5).map(|_| rng.gen::<u64>().to_string()).collect();

    std::iter::from_fn(move || {
        if generated >= count {
            return None;
        }
        generated += 1;

        let created_at = random_date(time_range.0, time_range.1, 0.5);
        let path = random_el(PATHS, 0.5);
        let referrer = random_el(REFERRERS, 0.5);
        let platform = random_el(PLATFORMS, -0.5);
        let browser = random_el(BROWSERS, -0.5);
        let mobile = rng.gen_bool(0.7);
        let (city, country) = random_el(CITIES, 0.5);

        Some(Event {
            browser: if browser.is_empty() { None } else { Some(browser.to_string()) },
            city: if city.is_empty() { None } else { Some(city.to_string()) },
            country: if country.is_empty() { None } else { Some(country.to_string()) },
            created_at,
            entity_id: entity_id.clone(),
            event: "pageview".to_string(),
            fqdn: Some(fqdn.clone()),
            mobile: Some(mobile),
            platform: if platform.is_empty() { None } else { Some(platform.to_string()) },
            referrer: if referrer.is_empty() { None } else { Some(referrer.to_string()) },
            path: Some(path.to_string()),
            visitor_id: random_el(&visitor_ids, 0.7).to_string(),
        })
    })
}

fn random_date(min: OffsetDateTime, max: OffsetDateTime, scale: f64) -> OffsetDateTime {
    let mut rng = rand::thread_rng();
    let uniform_random: f64 = rng.gen();
    let weighted_random = (uniform_random.powf(1.0 - scale)).min(1.0);
    let duration = max - min;
    let duration_seconds = duration.as_seconds_f64();
    let weighted_duration_seconds = duration_seconds * weighted_random;
    let weighted_duration = time::Duration::seconds(weighted_duration_seconds as i64);
    min + weighted_duration
}

fn random_el<T>(slice: &[T], scale: f64) -> &T {
    let mut rng = rand::thread_rng();
    let len = slice.len();

    assert!(len != 0, "Cannot choose from an empty slice");

    let uniform_random: f64 = rng.gen();
    let weighted_random = (uniform_random.powf(1.0 - scale)).min(1.0);
    let index = (weighted_random * (len as f64)) as usize;
    &slice[index.min(len - 1)]
}
