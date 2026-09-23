use crate::app::{SqlitePool, models};
use anyhow::Result;
use chrono::{DateTime, Utc};

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
        let mut stmt = conn
            .prepare_cached("insert into sessions (id, username, expires_at) values (:id, :username, :expires_at)")?;
        stmt.execute(rusqlite::named_params! {
            ":id": session_id,
            ":username": username,
            ":expires_at": expires_at,
        })?;
        Ok(())
    }

    /// Get the user associated with a session ID, if the session is still valid
    /// Returns `None` if the session is expired
    pub fn get(&self, session_id: &str) -> Result<Option<models::User>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare_cached(
            r#"--sql
            select u.username, u.role, u.projects
            from sessions s
            join users u
            on lower(u.username) = lower(s.username)
            where
                s.id = :session_id
                and s.expires_at > :now
        "#,
        )?;

        let user = stmt.query_row(rusqlite::named_params! { ":session_id": session_id, ":now": Utc::now() }, |row| {
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

    /// Expire a session
    pub fn delete(&self, session_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update sessions set expires_at = :expires_at where id = :id")?;
        stmt.execute(rusqlite::named_params! { ":expires_at": Utc::now(), ":id": session_id })?;
        Ok(())
    }
}
