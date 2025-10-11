use chrono::{DateTime, Utc};
use eyre::Result;
use refinery::{Migration, error::WrapMigrationError};
use refinery_core::{
    Migrate,
    traits::sync::{Query, Transaction},
};
use std::{ops::DerefMut, time::SystemTime};

pub struct DuckDBConnection<T: DerefMut<Target = duckdb::Connection>>(pub T);
impl<T: DerefMut<Target = duckdb::Connection>> From<T> for DuckDBConnection<T> {
    fn from(conn: T) -> Self {
        Self(conn)
    }
}

impl<Conn: DerefMut<Target = duckdb::Connection>> Transaction for DuckDBConnection<Conn> {
    type Error = duckdb::Error;
    fn execute<'a, T: Iterator<Item = &'a str>>(&mut self, mut queries: T) -> std::result::Result<usize, Self::Error> {
        let transaction = self.0.transaction()?;
        let count = queries.try_fold(0, |count, query| {
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
                let checksum: u64 = row.get(3)?;
                let applied_on: SystemTime = applied_on.into();

                Ok(Migration::applied(version, name, applied_on.into(), checksum))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(applied)
    }
}

impl<T: DerefMut<Target = duckdb::Connection>> Migrate for DuckDBConnection<T> {
    fn assert_migrations_table(&mut self, migration_table_name: &str) -> std::result::Result<usize, refinery::Error> {
        let query = format!(
            "create table if not exists {migration_table_name} (
                version int primary key,
                name text not null,
                applied_on timestamp not null,
                checksum text not null
            )"
        );

        self.execute(std::iter::once(query.as_str())).migration_err("error asserting migrations table", None)?;
        Ok(0)
    }
}
