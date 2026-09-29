use crate::app::{SqlitePool, models};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;
use std::collections::HashSet;

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

        let user = conn
            .query_row(
                "select u.username, u.role from sessions s join users u on lower(u.username) = lower(s.username) where s.id = ? and s.expires_at > ?",
                rusqlite::params![session_id, Utc::now()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((username, role)) = user else { return Ok(None) };

        let mut teams = conn.prepare_cached(
            "select t.all_projects, t.all_entities, t.permissions_json,
                (select json_group_array(project_id) from team_projects where team_id = t.id),
                (select json_group_array(entity_id) from team_entities where team_id = t.id)
             from teams t join team_users tu on tu.team_id = t.id where tu.username = ?",
        )?;
        let rows = teams.query_map([&username], |row| {
            Ok((
                row.get::<_, bool>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut projects = HashSet::new();
        let mut entities = HashSet::new();
        let mut all_projects = false;
        let mut all_entities = false;
        let mut permissions = HashSet::new();
        for row in rows {
            let (team_all_projects, team_all_entities, granted, project_ids, entity_ids) = row?;
            if serde_json::from_str::<HashSet<models::AccessPermission>>(&granted)?
                .contains(&models::AccessPermission::ProjectRead)
            {
                permissions.insert(models::AccessPermission::ProjectRead);
                all_projects |= team_all_projects;
                if !team_all_projects {
                    projects.extend(serde_json::from_str::<Vec<String>>(&project_ids)?);
                }
            }
            all_entities |= team_all_entities;
            if !team_all_entities {
                entities.extend(serde_json::from_str::<Vec<String>>(&entity_ids)?);
            }
        }

        Ok(Some(models::User {
            username,
            role: role.try_into().unwrap_or_default(),
            access: models::Access {
                projects: if all_projects {
                    models::AccessScope::All
                } else {
                    models::AccessScope::Selected(projects.into_iter().collect())
                },
                entities: if all_entities {
                    models::AccessScope::All
                } else {
                    models::AccessScope::Selected(entities.into_iter().collect())
                },
                permissions,
            },
        }))
    }

    /// Revoke all sessions belonging to a user.
    pub fn revoke_user(&self, username: &str) -> Result<()> {
        self.pool.get()?.execute("delete from sessions where lower(username) = ?", [username.to_lowercase()])?;
        Ok(())
    }

    /// Expire a session
    pub fn delete(&self, session_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update sessions set expires_at = :expires_at where id = :id")?;
        stmt.execute(rusqlite::named_params! { ":expires_at": Utc::now(), ":id": session_id })?;
        Ok(())
    }
}
