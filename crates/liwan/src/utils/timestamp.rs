use jiff::{Timestamp, civil::DateTime, tz::TimeZone};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use std::borrow::Cow;

pub(crate) struct TimestampSchema;

impl JsonSchema for TimestampSchema {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "DateTime".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string", "format": "date-time" })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SqlTimestamp(pub Timestamp);

impl duckdb::ToSql for SqlTimestamp {
    fn to_sql(&self) -> duckdb::Result<duckdb::types::ToSqlOutput<'_>> {
        use duckdb::types::{TimeUnit, ToSqlOutput, Value};
        let micros = self.0.as_nanosecond().div_euclid(1_000) as i64;
        Ok(ToSqlOutput::Owned(Value::Timestamp(TimeUnit::Microsecond, micros)))
    }
}

impl duckdb::types::FromSql for SqlTimestamp {
    fn column_result(value: duckdb::types::ValueRef<'_>) -> duckdb::types::FromSqlResult<Self> {
        use duckdb::types::{FromSqlError, TimeUnit, ValueRef};
        let timestamp = match value {
            ValueRef::Timestamp(TimeUnit::Second, x) => Timestamp::from_second(x),
            ValueRef::Timestamp(TimeUnit::Millisecond, x) => Timestamp::from_millisecond(x),
            ValueRef::Timestamp(TimeUnit::Microsecond, x) => Timestamp::from_microsecond(x),
            ValueRef::Timestamp(TimeUnit::Nanosecond, x) => Timestamp::from_nanosecond(i128::from(x)),
            _ => return Err(FromSqlError::InvalidType),
        };
        timestamp.map(Self).map_err(|err| FromSqlError::Other(Box::new(err)))
    }
}

impl rusqlite::ToSql for SqlTimestamp {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        // SQLite compares these as text, so retain the existing separator and offset.
        let nanos = self.0.subsec_nanosecond();
        let format = if nanos == 0 {
            "%F %T%:z"
        } else if nanos % 1_000_000 == 0 {
            "%F %T%.3f%:z"
        } else if nanos % 1_000 == 0 {
            "%F %T%.6f%:z"
        } else {
            "%F %T%.9f%:z"
        };
        Ok(self.0.strftime(format).to_string().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_preserves_legacy_timestamp_format() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let timestamp = SqlTimestamp("2026-01-01T12:00:00.12Z".parse().unwrap());
        let stored: String = conn.query_row("select ?", [timestamp], |row| row.get(0)).unwrap();
        assert_eq!(stored, "2026-01-01 12:00:00.120+00:00");

        for value in ["2026-01-01 12:00:00.120+00:00", "2026-01-01 12:00:00.120"] {
            let decoded: SqlTimestamp = conn.query_row("select ?", [value], |row| row.get(0)).unwrap();
            assert_eq!(decoded.0, timestamp.0);
        }
    }

    #[test]
    fn duckdb_preserves_microsecond_precision_before_epoch() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let timestamp = SqlTimestamp("1969-12-31T23:59:59.9999989Z".parse().unwrap());
        let decoded: SqlTimestamp = conn.query_row("select ?::timestamp", [timestamp], |row| row.get(0)).unwrap();
        assert_eq!(decoded.0, "1969-12-31T23:59:59.999998Z".parse().unwrap());
    }
}

impl rusqlite::types::FromSql for SqlTimestamp {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        use rusqlite::types::{FromSqlError, ValueRef};

        let timestamp = match value {
            ValueRef::Integer(x) => Timestamp::from_second(x),
            _ => {
                let text = value.as_str()?;
                text.parse::<Timestamp>()
                    .or_else(|_| text.parse::<DateTime>()?.to_zoned_datetime(TimeZone::UTC).map(|dt| dt.timestamp()))
            }
        };
        timestamp.map(Self).map_err(FromSqlError::other)
    }
}
