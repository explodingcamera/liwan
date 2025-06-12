use crate::app::{SqlitePool, models};
use chrono::{DateTime, Utc};
use eyre::Result;
use rusqlite::params;

#[derive(Clone)]
pub struct LiwanSessions {
    pool: SqlitePool,
}

impl LiwanSessions {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new session
    pub fn create(&self, session_id: &str, username: &str, expires_at: DateTime<Utc>) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("insert into sessions (id, username, expires_at) values (?, ?, ?)")?;
        stmt.execute(rusqlite::params![session_id, username, expires_at])?;
        Ok(())
    }

    /// Get the user associated with a session ID, if the session is still valid.
    /// Returns None if the session is expired
    pub fn get(&self, session_id: &str) -> Result<Option<models::User>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare_cached(
            r#"--sql
            select u.username, u.role, u.projects
            from sessions s
            join users u
            on lower(u.username) = lower(s.username)
            where
                s.id = ?
                and s.expires_at > ?
        "#,
        )?;

        let user = stmt.query_row(params![session_id, Utc::now()], |row| {
            Ok(models::User {
                username: row.get("username")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                projects: row
                    .get::<_, String>("projects")?
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect(),
            })
        });

        user.map(Some).or_else(
            |err| {
                if err == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(err.into()) }
            },
        )
    }

    /// Delete a session
    pub fn delete(&self, session_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update sessions set expires_at = ? where id = ?")?;
        stmt.execute(rusqlite::params![Utc::now(), session_id])?;
        Ok(())
    }
}
