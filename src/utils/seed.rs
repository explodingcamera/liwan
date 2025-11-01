use chrono::{DateTime, Duration, Utc};
use rand::Rng;

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
const UTM_CAMPAIGNS: &[&str] = &["", "summer_sale", "black_friday", "christmas", "new_year"];
const UTM_CONTENTS: &[&str] = &["", "banner", "sidebar", "footer", "popup"];
const UTM_MEDIUMS: &[&str] = &["", "cpc", "organic", "referral", "email"];
const UTM_SOURCES: &[&str] = &["", "google", "bing", "facebook", "twitter"];
const UTM_TERMS: &[&str] = &["", "liwan", "analytics", "tracking", "web"];

pub fn random_events(
    time_range: (DateTime<Utc>, DateTime<Utc>),
    entity_id: &str,
    fqdn: &str,
    count: usize,
) -> impl Iterator<Item = Event> {
    let mut rng = rand::rng();
    let mut generated = 0usize;
    let entity_id = entity_id.to_string();
    let fqdn = fqdn.to_string();

    // Keep the visitor pool relatively small compared to event count so we
    // get session continuity instead of switching visitors constantly.
    // Use count/20 but cap to a reasonable ceiling to avoid huge visitor pools.
    let mut visitors_count = (count / 20).max(1);
    visitors_count = visitors_count.min(100_000);
    let visitor_ids: Vec<String> = (0..visitors_count).map(|_| rng.random::<u64>().to_string()).collect();

    let total_seconds = (time_range.1 - time_range.0).num_seconds().max(1) as f64;
    // (mean_interval is computed below for streaming inter-arrival sampling)

    let jitter = (rng.random_range(0.0..1.0) * 0.02 * total_seconds) as i64;
    let mut current_time = time_range.0 + Duration::seconds(jitter);

    let mut current_visitor_idx: Option<usize> = None;

    std::iter::from_fn(move || {
        if generated >= count {
            return None;
        }

        let remaining_events = (count - generated) as f64;
        let rem_seconds = (time_range.1 - current_time).num_seconds().max(0) as f64;
        if rem_seconds <= 0.0 {
            return None;
        }

        let target_mean = (rem_seconds / remaining_events).max(0.001);

        let next_visitor_idx = if let Some(idx) = current_visitor_idx {
            if rng.random_range(0.0f64..1.0f64) < 0.85f64 {
                idx
            } else {
                (rng.random::<u64>() as usize) % visitor_ids.len()
            }
        } else {
            (rng.random::<u64>() as usize) % visitor_ids.len()
        };
        let is_switch = match current_visitor_idx {
            Some(prev) => next_visitor_idx != prev,
            None => true,
        };

        let u1 = rng.random_range(1e-12f64..1.0f64);
        let u2 = rng.random_range(0.0f64..1.0f64);
        let z = (-2.0f64 * u1.ln()).sqrt() * (2.0f64 * std::f64::consts::PI * u2).cos();
        let sigma = 0.5f64;
        let mut multiplier = (sigma * z).exp();
        multiplier = multiplier.clamp(0.5f64, 2.5f64);

        let mut inter_s = target_mean * multiplier;

        if is_switch && rng.random_range(0.0f64..1.0f64) < 0.10f64 {
            inter_s += rng.random_range(10.0f64..300.0f64);
        }

        let adv_ms = (inter_s * 1000.0).round() as i64;
        current_time += Duration::milliseconds(adv_ms.max(1));

        if current_time > time_range.1 {
            return None;
        }

        let visitor_idx = next_visitor_idx;
        current_visitor_idx = Some(visitor_idx);
        generated += 1;

        let path = random_el(PATHS, 0.8);
        let referrer = random_el(REFERRERS, 0.9);
        let platform = random_el(PLATFORMS, -0.3);
        let browser = random_el(BROWSERS, 0.0);
        let mobile = rng.random_bool(0.48);
        let (city, country) = random_el(CITIES, 0.8);

        Some(Event {
            browser: if browser.is_empty() { None } else { Some(browser.to_string()) },
            city: if city.is_empty() { None } else { Some(city.to_string()) },
            country: if country.is_empty() { None } else { Some(country.to_string()) },
            created_at: current_time,
            entity_id: entity_id.clone(),
            event: "pageview".to_string(),
            fqdn: Some(fqdn.clone()),
            mobile: Some(mobile),
            platform: if platform.is_empty() { None } else { Some(platform.to_string()) },
            referrer: if referrer.is_empty() { None } else { Some(referrer.to_string()) },
            path: Some(path.to_string()),
            visitor_id: visitor_ids[visitor_idx].clone(),
            utm_campaign: Some(random_el(UTM_CAMPAIGNS, 0.6).to_string()),
            utm_content: Some(random_el(UTM_CONTENTS, 0.6).to_string()),
            utm_medium: Some(random_el(UTM_MEDIUMS, 0.6).to_string()),
            utm_source: Some(random_el(UTM_SOURCES, 0.6).to_string()),
            utm_term: Some(random_el(UTM_TERMS, 0.6).to_string()),
        })
    })
}

fn random_el<T>(slice: &[T], scale: f64) -> &T {
    let mut rng = rand::rng();
    let len = slice.len();

    assert!(len != 0, "Cannot choose from an empty slice");

    let uniform_random: f64 = rng.random();
    let weighted_random = (uniform_random.powf(1.0 - scale)).min(1.0);
    let index = (weighted_random * (len as f64)) as usize;
    &slice[index.min(len - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_counts_and_order() {
        let start = Utc::now();
        let end = start + Duration::hours(6);
        let want = 2_000usize;

        let events: Vec<_> = random_events((start, end), "entity", "example.com", want).collect();
        let got = events.len();

        assert!(
            got >= (want as f64 * 0.7) as usize && got <= (want as f64 * 1.3) as usize,
            "generated {} events, expected ≈{} (±30%)",
            got,
            want
        );

        for i in 1..events.len() {
            assert!(
                events[i].created_at >= events[i - 1].created_at,
                "events not ordered at idx {}: {} before {}",
                i,
                events[i - 1].created_at,
                events[i].created_at
            );
        }

        if events.len() >= 2 {
            let mut last_ms = events[0].created_at.timestamp_millis();
            let mut intervals = std::collections::HashSet::new();
            for ev in &events[1..] {
                let now_ms = ev.created_at.timestamp_millis();
                intervals.insert((now_ms - last_ms).abs());
                last_ms = now_ms;
            }
            assert!(intervals.len() > 1, "intervals show no variance");
        }
    }
}
