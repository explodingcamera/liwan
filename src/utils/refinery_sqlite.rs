use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use refinery::Migration;
use refinery_core::traits::r#async::{AsyncMigrate, AsyncQuery, AsyncTransaction};
use sqlx::{AssertSqlSafe, Row, SqlitePool};

pub struct SqlxConnection<'a>(pub &'a SqlitePool);
impl<'a> From<&'a SqlitePool> for SqlxConnection<'a> {
    fn from(pool: &'a SqlitePool) -> Self {
        Self(pool)
    }
}

#[async_trait]
impl AsyncTransaction for SqlxConnection<'_> {
    type Error = sqlx::Error;
    async fn execute<'a, T: Iterator<Item = &'a str> + Send>(&mut self, queries: T) -> Result<usize, Self::Error> {
        let mut transaction = self.0.begin().await?;
        let mut count = 0;
        for query in queries {
            // migrations may contain multiple statements, so they can't be prepared
            sqlx::raw_sql(AssertSqlSafe(query.to_string())).execute(&mut *transaction).await?;
            count += 1;
        }
        transaction.commit().await?;
        Ok(count)
    }
}

#[async_trait]
impl AsyncQuery<Vec<Migration>> for SqlxConnection<'_> {
    async fn query(&mut self, query: &str) -> Result<Vec<Migration>, Self::Error> {
        let mut transaction = self.0.begin().await?;
        let rows = sqlx::query(AssertSqlSafe(query.to_string())).fetch_all(&mut *transaction).await?;
        transaction.commit().await?;

        let mut applied = Vec::with_capacity(rows.len());
        for row in rows {
            let version: i32 = row.try_get(0)?;
            let name: String = row.try_get(1)?;
            let applied_on: DateTime<Utc> = row.try_get(2)?;
            let applied_on: SystemTime = applied_on.into();
            let checksum: String = row.try_get(3)?;

            applied.push(Migration::applied(
                version,
                name,
                applied_on.into(),
                checksum.parse::<u64>().expect("checksum must be a valid u64"),
            ));
        }
        Ok(applied)
    }
}

impl AsyncMigrate for SqlxConnection<'_> {}
