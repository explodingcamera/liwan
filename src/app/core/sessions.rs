use crate::app::{SqlitePool, models};
use crate::utils::sqlite::timestamp;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

#[derive(Clone)]
pub struct LiwanSessions {
    pool: SqlitePool,
}

impl LiwanSessions {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new session
    pub async fn create(&self, session_id: &str, username: &str, expires_at: DateTime<Utc>) -> Result<()> {
        sqlx::query("insert into sessions (id, username, expires_at) values (?, ?, ?)")
            .bind(session_id)
            .bind(username)
            .bind(timestamp(expires_at))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get the user associated with a session ID, if the session is still valid
    /// Returns `None` if the session is expired
    pub async fn get(&self, session_id: &str) -> Result<Option<models::User>> {
        let row = sqlx::query(
            r#"--sql
            select u.username, u.role, u.projects
            from sessions s
            join users u
            on lower(u.username) = lower(s.username)
            where
                s.id = ?
                and s.expires_at > ?
        "#,
        )
        .bind(session_id)
        .bind(timestamp(Utc::now()))
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(models::User {
            username: row.try_get("username")?,
            role: row.try_get::<String, _>("role")?.try_into().unwrap_or_default(),
            projects: row
                .try_get::<String, _>("projects")?
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        }))
    }

    /// Expire a session
    pub async fn delete(&self, session_id: &str) -> Result<()> {
        sqlx::query("update sessions set expires_at = ? where id = ?")
            .bind(timestamp(Utc::now()))
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
