use crate::app::{SqlitePool, models};
use crate::utils::hash::{hash_password, verify_password};
use crate::utils::validate;
use anyhow::{Result, bail};
use rusqlite::OptionalExtension;

#[derive(Clone)]
pub struct LiwanUsers {
    pool: SqlitePool,
}

impl LiwanUsers {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Check whether a user's password is correct
    pub fn check_login(&self, username: &str, password: &str) -> Result<bool> {
        let username = username.to_lowercase();
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select password_hash from users where username = ?")?;
        let hash: Option<String> = stmt.query_row([username], |row| row.get(0))?;
        Ok(hash.is_some_and(|hash| verify_password(password, &hash).is_ok()))
    }

    /// Get a user by username
    pub fn get(&self, username: &str) -> Result<models::User> {
        let username = username.to_lowercase();
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select username, role from users where username = ?")?;
        let user = stmt.query_row([username], |row| {
            Ok(models::User {
                username: row.get("username")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                access: models::Access::default(),
            })
        });
        user.map_err(|_| anyhow::anyhow!("user not found"))
    }

    /// Get all users
    pub fn all(&self) -> Result<Vec<models::User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select username, role from users")?;
        let users = stmt.query_map([], |row| {
            Ok(models::User {
                username: row.get("username")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                access: models::Access::default(),
            })
        })?;
        Ok(users.collect::<Result<Vec<models::User>, rusqlite::Error>>()?)
    }

    /// Create a new user
    pub fn create(&self, username: &str, password: &str, role: models::UserRole) -> Result<()> {
        if !validate::is_valid_username(username) {
            bail!("invalid username");
        }
        let username = username.to_lowercase();
        let password_hash = hash_password(password)?;
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "insert into users (username, password_hash, role) values (:username, :password_hash, :role)",
        )?;
        stmt.execute(rusqlite::named_params! {
            ":username": username,
            ":password_hash": password_hash,
            ":role": role.to_string(),
        })?;
        Ok(())
    }

    /// Update a user's role and team memberships atomically.
    pub fn update(&self, username: &str, role: models::UserRole, team_ids: &[String]) -> Result<bool> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        if tx.execute("update users set role = ? where username = ?", rusqlite::params![role.to_string(), username])?
            == 0
        {
            return Ok(false);
        }
        let team_ids_json = serde_json::to_string(team_ids)?;
        let missing: Option<String> = tx
            .query_row(
                "select value from json_each(?) where value not in (select id from teams) limit 1",
                [&team_ids_json],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(team_id) = missing {
            bail!("team not found: {team_id}");
        }
        tx.execute("delete from team_users where username = ?", [username])?;
        tx.execute(
            "insert into team_users (team_id, username) select distinct value, ? from json_each(?)",
            rusqlite::params![username, team_ids_json],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Update a password and revoke other sessions. CLI resets may omit the current password and session.
    pub fn update_password(
        &self,
        username: &str,
        password: &str,
        current_password: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<bool> {
        let username = username.to_lowercase();
        let Some(previous_hash) = self
            .pool
            .get()?
            .query_row("select password_hash from users where username = ?", [&username], |row| {
                row.get::<_, Option<String>>(0)
            })
            .optional()?
        else {
            return Ok(false);
        };
        if let Some(current_password) = current_password
            && previous_hash.as_deref().is_none_or(|hash| verify_password(current_password, hash).is_err())
        {
            return Ok(false);
        }
        let password_hash = hash_password(password)?;
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        if tx.execute(
            "update users set password_hash = ? where username = ? and password_hash is ?",
            rusqlite::params![password_hash, username, previous_hash],
        )? == 0
        {
            return Ok(false);
        }
        tx.execute(
            "delete from sessions where lower(username) = ? and id != ?",
            [&username, session_id.unwrap_or_default()],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Delete a user
    pub fn delete(&self, username: &str) -> Result<()> {
        let username = username.to_lowercase();
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute("delete from sessions where username = ?", [&username])?;
        transaction.execute("delete from team_users where username = ?", [&username])?;
        transaction.execute("delete from users where username = ?", [&username])?;
        transaction.commit()?;
        Ok(())
    }
}
