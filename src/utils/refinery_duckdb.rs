use chrono::{DateTime, Utc};
use eyre::Result;
use refinery::{error::WrapMigrationError, Migration};
use refinery_core::{
    traits::sync::{Query, Transaction},
    Migrate,
};
use std::ops::DerefMut;

pub(crate) struct DuckDBConnection<T: DerefMut<Target = duckdb::Connection>>(pub(crate) T);
impl<T: DerefMut<Target = duckdb::Connection>> From<T> for DuckDBConnection<T> {
    fn from(conn: T) -> Self {
        DuckDBConnection(conn)
    }
}

impl<T: DerefMut<Target = duckdb::Connection>> Transaction for DuckDBConnection<T> {
    type Error = duckdb::Error;
    fn execute(&mut self, queries: &[&str]) -> Result<usize, Self::Error> {
        let transaction = self.0.transaction()?;
        let count = queries.iter().try_fold(0, |count, query| {
            transaction.execute_batch(query)?;
            Ok::<_, Self::Error>(count + 1)
        })?;
        transaction.commit()?;
        Ok(count)
    }
}

impl<T: DerefMut<Target = duckdb::Connection>> Query<Vec<Migration>> for DuckDBConnection<T> {
    fn query(&mut self, query: &str) -> Result<Vec<Migration>, Self::Error> {
        let mut stmt = self.0.prepare(query)?;
        let applied: Vec<Migration> = stmt
            .query_map([], |row| {
                let version = row.get(0)?;
                let name: String = row.get(1)?;
                let applied_on: DateTime<Utc> = row.get(2)?;
                let applied_on = time::OffsetDateTime::from_unix_timestamp(applied_on.timestamp()).unwrap();
                let checksum: u64 = row.get(3)?;
                Ok(Migration::applied(version, name, applied_on, checksum))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(applied)
    }
}

impl<T: DerefMut<Target = duckdb::Connection>> Migrate for DuckDBConnection<T> {
    fn assert_migrations_table(&mut self, migration_table_name: &str) -> std::result::Result<usize, refinery::Error> {
        let query = format!(
            "create table if not exists {} (
                version int primary key,
                name text not null,
                applied_on timestamp not null,
                checksum text not null
            )",
            migration_table_name
        );
        self.execute(&[&query]).migration_err("error asserting migrations table", None)?;
        Ok(0)
    }
}
