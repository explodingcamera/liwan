use crate::app::{SqlitePool, models};
use crate::utils::hash::{hash_password, verify_password};
use crate::utils::validate;
use anyhow::{Result, bail};
use sqlx::{Row, sqlite::SqliteRow};
use tokio::task::spawn_blocking;

#[derive(Clone)]
pub struct LiwanUsers {
    pool: SqlitePool,
}

/// Hash a password without blocking a runtime worker thread
async fn hash_password_blocking(password: &str) -> Result<String> {
    let password = password.to_string();
    spawn_blocking(move || hash_password(&password)).await?
}

fn to_user(row: &SqliteRow) -> Result<models::User, sqlx::Error> {
    Ok(models::User {
        username: row.try_get("username")?,
        role: row.try_get::<String, _>("role")?.try_into().unwrap_or_default(),
        projects: row
            .try_get::<String, _>("projects")?
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    })
}

impl LiwanUsers {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Check whether a user's password is correct
    pub async fn check_login(&self, username: &str, password: &str) -> Result<bool> {
        let username = username.to_lowercase();
        let hash: String = sqlx::query_scalar("select password_hash from users where username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await?;
        // password hashing is expensive, so it shouldn't run on a runtime worker thread
        let password = password.to_string();
        Ok(spawn_blocking(move || verify_password(&password, &hash).is_ok()).await?)
    }

    /// Get a user by username
    pub async fn get(&self, username: &str) -> Result<models::User> {
        let username = username.to_lowercase();
        let user = sqlx::query("select username, password_hash, role, projects from users where username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .and_then(|row| to_user(&row));
        user.map_err(|_| anyhow::anyhow!("user not found"))
    }

    /// Get all users
    pub async fn all(&self) -> Result<Vec<models::User>> {
        let rows =
            sqlx::query("select username, password_hash, role, projects from users").fetch_all(&self.pool).await?;
        Ok(rows.iter().map(to_user).collect::<Result<Vec<models::User>, sqlx::Error>>()?)
    }

    /// Create a new user
    pub async fn create(
        &self,
        username: &str,
        password: &str,
        role: models::UserRole,
        projects: &[&str],
    ) -> Result<()> {
        if !validate::is_valid_username(username) {
            bail!("invalid username");
        }
        let username = username.to_lowercase();
        let password_hash = hash_password_blocking(password).await?;
        sqlx::query("insert into users (username, password_hash, role, projects) values (?, ?, ?, ?)")
            .bind(username)
            .bind(password_hash)
            .bind(role.to_string())
            .bind(projects.join(","))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update a user's role and project memberships
    pub async fn update(&self, username: &str, role: models::UserRole, projects: &[String]) -> Result<()> {
        sqlx::query("update users set role = ?, projects = ? where username = ?")
            .bind(role.to_string())
            .bind(projects.join(","))
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update a user's password
    pub async fn update_password(&self, username: &str, password: &str) -> Result<()> {
        let password_hash = hash_password_blocking(password).await?;
        sqlx::query("update users set password_hash = ? where username = ?")
            .bind(password_hash)
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a user
    pub async fn delete(&self, username: &str) -> Result<()> {
        sqlx::query("delete from users where username = ?").bind(username).execute(&self.pool).await?;
        Ok(())
    }
}
