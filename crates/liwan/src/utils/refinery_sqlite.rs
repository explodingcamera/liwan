use std::{ops::DerefMut, time::SystemTime};

use chrono::{DateTime, Utc};
use refinery::Migration;
use refinery_core::traits::sync::{Migrate, Query, Transaction};
use rusqlite::{Connection, Error as RqlError};

pub struct RqlConnection<T: DerefMut<Target = Connection>>(pub T);
impl<T: DerefMut<Target = Connection>> From<T> for RqlConnection<T> {
    fn from(conn: T) -> Self {
        Self(conn)
    }
}

fn query_applied_migrations(transaction: &rusqlite::Transaction, query: &str) -> Result<Vec<Migration>, RqlError> {
    let mut stmt = transaction.prepare(query)?;
    let mut rows = stmt.query([])?;
    let mut applied = Vec::new();
    while let Some(row) = rows.next()? {
        let version = row.get(0)?;
        let applied_on: DateTime<Utc> = row.get(2)?;
        let applied_on: SystemTime = applied_on.into();
        let checksum: String = row.get(3)?;

        applied.push(Migration::applied(
            version,
            row.get(1)?,
            applied_on.into(),
            checksum.parse::<u64>().expect("checksum must be a valid u64"),
        ));
    }
    Ok(applied)
}

impl<Conn: DerefMut<Target = Connection>> Transaction for RqlConnection<Conn> {
    type Error = RqlError;
    fn execute<'a, T: Iterator<Item = &'a str>>(&mut self, queries: T) -> Result<usize, Self::Error> {
        let transaction = self.0.transaction()?;
        let mut count = 0;
        for query in queries {
            transaction.execute_batch(query)?;
            count += 1;
        }
        transaction.commit()?;
        Ok(count)
    }
}

impl<T: DerefMut<Target = Connection>> Query<Vec<Migration>> for RqlConnection<T> {
    fn query(&mut self, query: &str) -> Result<Vec<Migration>, Self::Error> {
        let transaction = self.0.transaction()?;
        let applied = query_applied_migrations(&transaction, query)?;
        transaction.commit()?;
        Ok(applied)
    }
}

impl<T: DerefMut<Target = Connection>> Migrate for RqlConnection<T> {}
