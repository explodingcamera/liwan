use crate::app::SqlitePool;
use eyre::Result;

#[derive(Clone)]
pub(crate) struct LiwanSessions {
    pool: SqlitePool,
}

impl LiwanSessions {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub(crate) fn session_create(
        &self,
        session_id: &str,
        username: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("insert into sessions (id, username, expires_at) values (?, ?, ?)")?;
        stmt.execute(rusqlite::params![session_id, username, expires_at])?;
        Ok(())
    }

    /// Get the username associated with a session ID, if the session is still valid.
    /// Returns None if the session is expired
    pub(crate) fn session_get(&self, session_id: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("select username, expires_at from sessions where id = ?")?;
        let (username, expires_at): (String, chrono::DateTime<chrono::Utc>) =
            stmt.query_row([session_id], |row| Ok((row.get("username")?, row.get("expires_at")?)))?;
        if expires_at < chrono::Utc::now() {
            return Ok(None);
        }
        Ok(Some(username))
    }

    pub(crate) fn session_delete(&self, session_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update sessions set expires_at = ? where id = ?")?;
        stmt.execute(rusqlite::params![chrono::Utc::now(), session_id])?;
        Ok(())
    }
}
