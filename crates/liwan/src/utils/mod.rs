pub(crate) mod duckdb;
pub(crate) mod geo;
pub mod geoip_headers;
pub(crate) mod hash;
pub mod ip_headers;
pub mod r2d2_sqlite;
pub(crate) mod referrer;
pub(crate) mod refinery_duckdb;
pub(crate) mod refinery_sqlite;
pub(crate) mod seed;
pub mod serde;
pub(crate) mod signals;
pub(crate) mod useragent;
pub(crate) mod validate;
pub(crate) mod writable;

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_hex_bytes() {
        assert_eq!(super::encode_hex(&[0, 15, 255]), "000fff");
    }
}
