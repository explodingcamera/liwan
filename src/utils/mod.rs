pub mod duckdb;
pub mod geo;
pub mod hash;
pub mod ip_headers;
pub mod r2d2_sqlite;
pub mod referrer;
pub mod refinery_duckdb;
pub mod refinery_sqlite;
pub mod seed;
pub mod signals;
pub mod useragent;
pub mod validate;
pub mod writable;

pub fn to_sorted<T: Clone + Ord>(v: &[T]) -> Vec<T> {
    let mut v = v.to_vec();
    v.sort_unstable();
    v
}
