use chrono::{DateTime, Utc};

pub(crate) mod geo;
pub(crate) mod hash;
pub(crate) mod referrer;
pub(crate) mod refinery_duckdb;
pub(crate) mod refinery_sqlite;
pub(crate) mod seed;
pub(crate) mod useragent;
pub(crate) mod validate;

pub(crate) trait TimeExt {
    fn to_time(&self) -> DateTime<Utc>;
}

pub(crate) type Timestamp = i64;

impl TimeExt for Timestamp {
    fn to_time(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(*self).unwrap_or_default()
    }
}
