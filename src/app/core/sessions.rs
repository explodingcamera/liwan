use crate::app::SqlitePool;
use eyre::Result;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct LiwanSessions {
    pool: SqlitePool,
}

impl LiwanSessions {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new session
    pub fn create(&self, session_id: &str, username: &str, expires_at: OffsetDateTime) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("insert into sessions (id, username, expires_at) values (?, ?, ?)")?;
        stmt.execute(rusqlite::params![session_id, username, expires_at])?;
        Ok(())
    }

    /// Get the username associated with a session ID, if the session is still valid.
    /// Returns None if the session is expired
    pub fn get(&self, session_id: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("select username, expires_at from sessions where id = ?")?;
        let (username, expires_at): (String, OffsetDateTime) =
            stmt.query_row([session_id], |row| Ok((row.get("username")?, row.get("expires_at")?)))?;
        if expires_at < OffsetDateTime::now_utc() {
            return Ok(None);
        }
        Ok(Some(username))
    }

    /// Delete a session
    pub fn delete(&self, session_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update sessions set expires_at = ? where id = ?")?;
        stmt.execute(rusqlite::params![OffsetDateTime::now_utc(), session_id])?;
        Ok(())
    }
}
