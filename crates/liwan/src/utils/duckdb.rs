use duckdb::ToSql;

/// Storage format Liwan targets, verified against the bundled DuckDB's `latest` in tests.
pub(crate) const LATEST_STORAGE_FORMAT: &str = "v2.0.0";

/// Identify newer-format errors, which DuckDB reports through generic error codes.
pub(crate) fn requires_newer_duckdb(message: &str) -> bool {
    message.contains("file was written with a storage version greater than the latest version supported")
        || message.contains("The database file was created with a newer version of DuckDB.")
}

/// Read the on-disk storage format without DuckDB's compatibility suffix.
pub(crate) fn storage_format(conn: &duckdb::Connection, database: Option<&str>) -> duckdb::Result<String> {
    conn.query_row(
        "SELECT rtrim(tags['storage_version'], '+') FROM duckdb_databases()
         WHERE database_name = coalesce(?, current_database())",
        [database],
        |row| row.get(0),
    )
}

#[derive(Default)]
pub struct ParamVec<'a>(Vec<Box<dyn ToSql + 'a>>);

impl<'a> ParamVec<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<T: ToSql + 'a>(&mut self, value: T) {
        self.0.push(Box::new(value));
    }

    pub fn extend_from_params(&mut self, params: Self) {
        self.0.extend(params.0);
    }

    pub fn extend<T: ToSql + 'a>(&mut self, iter: impl IntoIterator<Item = T>) {
        self.0.extend(iter.into_iter().map(|v| Box::new(v) as Box<dyn ToSql + 'a>));
    }
}

impl<'a> IntoIterator for ParamVec<'a> {
    type Item = Box<dyn ToSql + 'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// # Panics
/// Panics if `count` is 0 - this needs to be handled by the caller
pub fn repeat_vars(count: usize) -> String {
    assert_ne!(count, 0);
    let mut s = "?,".repeat(count);
    // Remove trailing comma
    s.pop();
    s
}
