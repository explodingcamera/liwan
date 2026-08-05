use chrono::{DateTime, Utc};

/// Encode a timestamp the same way `rusqlite` did (`2024-01-01 12:00:00+00:00`).
///
/// `sqlx` would encode timestamps as RFC 3339 instead (using `T` as the date/time separator), which
/// decodes fine but doesn't compare correctly against timestamps written by previous versions of
/// liwan, since sqlite compares them as text.
pub fn timestamp(value: DateTime<Utc>) -> String {
    value.format("%F %T%.f%:z").to_string()
}

/// Create a `sqlx` decoding error for a column that couldn't be converted into the expected type
pub fn decode_err(column: &str, err: impl std::fmt::Display) -> sqlx::Error {
    sqlx::Error::ColumnDecode {
        index: column.to_string(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())),
    }
}
