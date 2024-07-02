use chrono::{DateTime, Utc};

pub mod hash;
pub mod referer;
pub mod refinery_duckdb;
pub mod ua;
pub mod validate;

pub trait TimeExt {
    fn to_time(&self) -> DateTime<Utc>;
}

pub type Timestamp = i64;

impl TimeExt for Timestamp {
    fn to_time(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(*self).unwrap_or_default()
    }
}
