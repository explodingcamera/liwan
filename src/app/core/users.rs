use crate::app::{models, SqlitePool};
use crate::utils::hash::{hash_password, verify_password};
use crate::utils::validate;
use eyre::{bail, Result};

#[derive(Clone)]
pub struct LiwanUsers {
    pool: SqlitePool,
}

impl LiwanUsers {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Check if a users password is correct
    pub fn check_login(&self, username: &str, password: &str) -> Result<bool> {
        let username = username.to_lowercase();
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select password_hash from users where username = ?")?;
        let hash: String = stmt.query_row([username], |row| row.get(0))?;
        Ok(verify_password(password, &hash).is_ok())
    }

    /// Get a user by username
    pub fn get(&self, username: &str) -> Result<models::User> {
        let username = username.to_lowercase();
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select username, password_hash, role, projects from users where username = ?")?;
        let user = stmt.query_row([username], |row| {
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
        user.map_err(|_| eyre::eyre!("user not found"))
    }

    /// Get all users
    pub fn all(&self) -> Result<Vec<models::User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select username, password_hash, role, projects from users")?;
        let users = stmt.query_map([], |row| {
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
        })?;
        Ok(users.collect::<Result<Vec<models::User>, rusqlite::Error>>()?)
    }

    /// Create a new user
    pub fn create(&self, username: &str, password: &str, role: models::UserRole, projects: &[&str]) -> Result<()> {
        if !validate::is_valid_username(username) {
            bail!("invalid username");
        }
        let username = username.to_lowercase();
        let password_hash = hash_password(password)?;
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("insert into users (username, password_hash, role, projects) values (?, ?, ?, ?)")?;
        stmt.execute([username, password_hash, role.to_string(), projects.join(",")])?;
        Ok(())
    }

    /// Update a user
    pub fn update(&self, username: &str, role: models::UserRole, projects: &[String]) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update users set role = ?, projects = ? where username = ?")?;
        stmt.execute([&role.to_string(), &projects.join(","), username])?;
        Ok(())
    }

    /// Update a user's password
    pub fn update_password(&self, username: &str, password: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let password_hash = hash_password(password)?;
        let mut stmt = conn.prepare_cached("update users set password_hash = ? where username = ?")?;
        stmt.execute([&password_hash, username])?;
        Ok(())
    }

    /// Delete a user
    pub fn delete(&self, username: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("delete from users where username = ?")?;
        stmt.execute([username])?;
        Ok(())
    }
}
