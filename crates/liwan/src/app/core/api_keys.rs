use std::collections::HashSet;

use anyhow::{Result, bail};
use chrono::Utc;
use rand::distr::{Alphanumeric, SampleString};
use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::app::{
    SqlitePool,
    models::{Access, AccessPermission, AccessScope, ApiKey, ApiKeyExpiration},
};

const KEY_PREFIX: &str = "liw_";

/// The permissions and entity access assigned to an authenticated API key.
pub struct ApiKeyAccess {
    pub id: String,
    pub access: Access,
}

/// Stores and verifies API keys.
#[derive(Clone)]
pub struct LiwanApiKeys {
    pool: SqlitePool,
}

impl LiwanApiKeys {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a key and returns its metadata and one-time plaintext value.
    pub fn create(
        &self,
        display_name: &str,
        entities: &AccessScope,
        projects: &AccessScope,
        permissions: &[AccessPermission],
        expiration: ApiKeyExpiration,
    ) -> Result<(ApiKey, String)> {
        let display_name = display_name.trim();
        validate_display_name(display_name)?;
        let secret = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let plaintext = format!("{KEY_PREFIX}{secret}");
        let id = Alphanumeric.sample_string(&mut rand::rng(), 16);
        let secret_hash = blake3::hash(secret.as_bytes()).to_hex().to_string();
        let permissions_json = serde_json::to_string(permissions)?;
        let created_at = Utc::now();
        let expires_at = expiration.expires_at();
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute(
            "insert into api_keys (id, secret_hash, display_name, permissions_json, created_at, expires_at) values (?, ?, ?, ?, ?, ?)",
            rusqlite::params![id, secret_hash, display_name, permissions_json, created_at, expires_at],
        )?;
        set_access(&tx, &id, entities, projects, permissions)?;
        tx.commit()?;

        Ok((
            ApiKey {
                id,
                display_name: display_name.to_string(),
                entities: entities.clone(),
                projects: projects.clone(),
                permissions: permissions.to_vec(),
                created_at,
                last_used_at: None,
                expires_at,
            },
            plaintext,
        ))
    }

    /// Lists all key metadata without exposing hashes.
    pub fn all(&self) -> Result<Vec<ApiKey>> {
        let conn = self.pool.get()?;
        query_keys(&conn, "", [])
    }

    /// Updates a key's display name, access, and permissions.
    pub fn update(
        &self,
        key_id: &str,
        display_name: &str,
        entities: &AccessScope,
        projects: &AccessScope,
        permissions: &[AccessPermission],
    ) -> Result<bool> {
        let display_name = display_name.trim();
        validate_display_name(display_name)?;
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        if !tx.prepare_cached("select 1 from api_keys where id = ? limit 1")?.exists([key_id])? {
            return Ok(false);
        }
        tx.execute("update api_keys set display_name = ? where id = ?", [display_name, key_id])?;
        set_access(&tx, key_id, entities, projects, permissions)?;
        tx.commit()?;
        Ok(true)
    }

    /// Deletes a key and its access assignments.
    pub fn delete(&self, key_id: &str) -> Result<bool> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from api_key_entities where key_id = ?", [key_id])?;
        tx.execute("delete from api_key_projects where key_id = ?", [key_id])?;
        let deleted = tx.execute("delete from api_keys where id = ?", [key_id])? > 0;
        tx.commit()?;
        Ok(deleted)
    }

    /// Replaces a key's secret and sets a new expiration, without changing its access.
    pub fn regenerate(&self, key_id: &str, expiration: ApiKeyExpiration) -> Result<Option<(ApiKey, String)>> {
        let secret = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let hash = blake3::hash(secret.as_bytes()).to_hex().to_string();
        let expires_at = expiration.expires_at();
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute(
            "update api_keys set secret_hash = ?, expires_at = ?, last_used_at = null where id = ?",
            rusqlite::params![hash, expires_at, key_id],
        )?;
        let Some(key) = query_keys(&tx, "where id = ?", [key_id])?.pop() else { return Ok(None) };
        tx.commit()?;
        Ok(Some((key, format!("{KEY_PREFIX}{secret}"))))
    }

    /// Moves a key's creation and expiration dates into the past for seed data.
    #[cfg(any(debug_assertions, test))]
    pub(crate) fn expire_for_seed(&self, key_id: &str) -> Result<()> {
        let now = Utc::now();
        self.pool.get()?.execute(
            "update api_keys set created_at = ?, expires_at = ? where id = ?",
            rusqlite::params![now - chrono::Duration::days(8), now - chrono::Duration::days(1), key_id],
        )?;
        Ok(())
    }

    /// Verifies a plaintext key and returns its current access.
    pub fn authenticate(&self, plaintext: &str) -> Result<Option<ApiKeyAccess>> {
        let Some(secret) = plaintext.strip_prefix(KEY_PREFIX) else { return Ok(None) };
        if secret.len() != 32
            || !secret.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Ok(None);
        }

        let conn = self.pool.get()?;
        let id = blake3::hash(secret.as_bytes()).to_hex();
        let access = conn
            .query_row(
                "select id,
                     (select json_group_array(entity_id) from api_key_entities where key_id = api_keys.id),
                     permissions_json, all_entities, all_projects,
                     (select json_group_array(project_id) from api_key_projects where key_id = api_keys.id)
                 from api_keys where secret_hash = ? and (expires_at is null or expires_at > ?)",
                rusqlite::params![id.as_str(), Utc::now()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, bool>(3)?,
                        row.get::<_, bool>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?;
        let Some((id, entities, permissions, all_entities, all_projects, projects)) = access else { return Ok(None) };
        let mut permitted_entities: HashSet<String> = serde_json::from_str(&entities)?;
        if !all_entities && all_projects {
            let mut stmt = conn.prepare_cached("select distinct entity_id from project_entities")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                permitted_entities.insert(row?);
            }
        } else if !all_entities {
            let mut stmt = conn.prepare_cached(
                "select distinct entity_id from project_entities where project_id in (select project_id from api_key_projects where key_id = ?)",
            )?;
            let rows = stmt.query_map([&id], |row| row.get::<_, String>(0))?;
            for row in rows {
                permitted_entities.insert(row?);
            }
        }
        conn.execute("update api_keys set last_used_at = ? where id = ?", rusqlite::params![Utc::now(), id])?;
        Ok(Some(ApiKeyAccess {
            id,
            access: Access {
                entities: if all_entities {
                    AccessScope::All
                } else {
                    AccessScope::Selected(permitted_entities.into_iter().collect())
                },
                projects: AccessScope::from_db(all_projects, &projects)?,
                permissions: serde_json::from_str(&permissions)?,
            },
        }))
    }
}

fn query_keys(conn: &Connection, filter: &str, params: impl rusqlite::Params) -> Result<Vec<ApiKey>> {
    let mut stmt = conn.prepare_cached(&format!(
        "select id, display_name, created_at, expires_at, last_used_at, permissions_json, all_entities,
            (select json_group_array(entity_id) from api_key_entities where key_id = api_keys.id), all_projects,
            (select json_group_array(project_id) from api_key_projects where key_id = api_keys.id)
         from api_keys {filter} order by created_at desc"
    ))?;
    let mut rows = stmt.query(params)?;
    let mut keys = Vec::new();
    while let Some(row) = rows.next()? {
        keys.push(ApiKey {
            id: row.get(0)?,
            display_name: row.get(1)?,
            created_at: row.get(2)?,
            expires_at: row.get(3)?,
            last_used_at: row.get(4)?,
            permissions: serde_json::from_str(&row.get::<_, String>(5)?)?,
            entities: AccessScope::from_db(row.get(6)?, &row.get::<_, String>(7)?)?,
            projects: AccessScope::from_db(row.get(8)?, &row.get::<_, String>(9)?)?,
        });
    }
    Ok(keys)
}

fn validate_display_name(display_name: &str) -> Result<()> {
    if display_name.is_empty() || display_name.len() > 100 {
        bail!("API key name must be between 1 and 100 characters");
    }
    Ok(())
}

fn set_access(
    tx: &Transaction<'_>,
    key_id: &str,
    entities: &AccessScope,
    projects: &AccessScope,
    permissions: &[AccessPermission],
) -> Result<()> {
    if permissions.iter().any(|permission| *permission != AccessPermission::EventsBatch) {
        bail!("permission not supported for API keys");
    }
    tx.execute("delete from api_key_entities where key_id = ?", [key_id])?;
    tx.execute("delete from api_key_projects where key_id = ?", [key_id])?;
    let mut entity_exists = tx.prepare_cached("select 1 from entities where id = ? limit 1")?;
    if let AccessScope::Selected(ids) = entities {
        for entity_id in ids {
            if !entity_exists.exists([entity_id])? {
                bail!("entity not found: {entity_id}");
            }
            tx.execute(
                "insert or ignore into api_key_entities (key_id, entity_id) values (?, ?)",
                [key_id, entity_id],
            )?;
        }
    }
    let mut project_exists = tx.prepare_cached("select 1 from projects where id = ? limit 1")?;
    if let AccessScope::Selected(ids) = projects {
        for project_id in ids {
            if !project_exists.exists([project_id])? {
                bail!("project not found: {project_id}");
            }
            tx.execute(
                "insert or ignore into api_key_projects (key_id, project_id) values (?, ?)",
                [key_id, project_id],
            )?;
        }
    }
    tx.execute(
        "update api_keys set permissions_json = ?, all_entities = ?, all_projects = ? where id = ?",
        rusqlite::params![
            serde_json::to_string(permissions)?,
            matches!(entities, AccessScope::All),
            matches!(projects, AccessScope::All),
            key_id
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{
            Liwan,
            models::{AccessPermission, AccessScope, ApiKeyExpiration, Entity, Project, ProjectVisibility},
        },
        config::Config,
    };

    #[test]
    fn key_access_rotation_and_deletion_apply_immediately() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        app.entities.create(&Entity { id: "docs".into(), display_name: "Docs".into() }, &[]).unwrap();
        app.entities.create(&Entity { id: "shop".into(), display_name: "Shop".into() }, &[]).unwrap();
        let (key, plaintext) = app
            .api_keys
            .create(
                "production",
                &AccessScope::Selected(vec!["docs".into()]),
                &AccessScope::Selected(vec![]),
                &[AccessPermission::EventsBatch],
                ApiKeyExpiration::Never,
            )
            .unwrap();

        let access = app.api_keys.authenticate(&plaintext).unwrap().unwrap();
        assert_eq!(access.id, key.id);
        assert_eq!(key.id.len(), 16);
        assert!(app.api_keys.all().unwrap()[0].last_used_at.is_some());
        assert!(access.access.can_access_entity("docs", AccessPermission::EventsBatch));
        assert!(!access.access.can_access_entity("shop", AccessPermission::EventsBatch));
        app.entities.delete("docs").unwrap();
        assert!(
            !app.api_keys
                .authenticate(&plaintext)
                .unwrap()
                .unwrap()
                .access
                .can_access_entity("docs", AccessPermission::EventsBatch)
        );
        app.api_keys
            .update(
                &key.id,
                "Production",
                &AccessScope::Selected(vec!["shop".into()]),
                &AccessScope::Selected(vec![]),
                &[AccessPermission::EventsBatch],
            )
            .unwrap();
        assert_eq!(app.api_keys.all().unwrap()[0].display_name, "Production");
        assert!(
            app.api_keys
                .authenticate(&plaintext)
                .unwrap()
                .unwrap()
                .access
                .can_access_entity("shop", AccessPermission::EventsBatch)
        );
        app.api_keys
            .update(
                &key.id,
                "Production",
                &AccessScope::Selected(vec!["shop".into()]),
                &AccessScope::Selected(vec![]),
                &[],
            )
            .unwrap();
        assert!(
            !app.api_keys
                .authenticate(&plaintext)
                .unwrap()
                .unwrap()
                .access
                .can_access_entity("shop", AccessPermission::EventsBatch)
        );
        let (rotated, replacement) = app.api_keys.regenerate(&key.id, ApiKeyExpiration::SevenDays).unwrap().unwrap();
        assert_eq!(rotated.id, key.id);
        assert!(rotated.expires_at.is_some());
        assert!(rotated.last_used_at.is_none());
        assert!(app.api_keys.authenticate(&plaintext).unwrap().is_none());
        assert!(app.api_keys.authenticate(&replacement).unwrap().is_some());
        app.api_keys
            .update(
                &key.id,
                "Production",
                &AccessScope::All,
                &AccessScope::Selected(vec![]),
                &[AccessPermission::EventsBatch],
            )
            .unwrap();
        assert!(
            app.api_keys
                .authenticate(&replacement)
                .unwrap()
                .unwrap()
                .access
                .can_access_entity("shop", AccessPermission::EventsBatch)
        );
        assert!(app.api_keys.delete(&key.id).unwrap());
        assert!(app.api_keys.authenticate(&plaintext).unwrap().is_none());
    }

    #[test]
    fn project_scope_and_expiration() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        for id in ["shop", "docs", "standalone"] {
            app.entities.create(&Entity { id: id.into(), display_name: id.into() }, &[]).unwrap();
        }
        for id in ["store", "site"] {
            app.projects
                .create(
                    &Project {
                        id: id.into(),
                        display_name: id.into(),
                        visibility: ProjectVisibility::Private,
                        secret: None,
                    },
                    &[],
                )
                .unwrap();
        }
        app.projects.update_entities("store", &["shop".into()]).unwrap();
        app.projects.update_entities("site", &["docs".into()]).unwrap();
        let (key, plaintext) = app
            .api_keys
            .create(
                "project key",
                &AccessScope::Selected(vec![]),
                &AccessScope::Selected(vec!["store".into()]),
                &[AccessPermission::EventsBatch],
                ApiKeyExpiration::ThirtyDays,
            )
            .unwrap();
        let access = app.api_keys.authenticate(&plaintext).unwrap().unwrap();
        assert!(access.access.can_access_entity("shop", AccessPermission::EventsBatch));
        assert!(!access.access.can_access_entity("docs", AccessPermission::EventsBatch));
        app.projects.update_entities("store", &["docs".into()]).unwrap();
        let access = app.api_keys.authenticate(&plaintext).unwrap().unwrap();
        assert!(!access.access.can_access_entity("shop", AccessPermission::EventsBatch));
        assert!(access.access.can_access_entity("docs", AccessPermission::EventsBatch));

        app.api_keys
            .update(
                &key.id,
                "all projects",
                &AccessScope::Selected(vec![]),
                &AccessScope::All,
                &[AccessPermission::EventsBatch],
            )
            .unwrap();
        let access = app.api_keys.authenticate(&plaintext).unwrap().unwrap();
        assert!(!access.access.can_access_entity("shop", AccessPermission::EventsBatch));
        assert!(access.access.can_access_entity("docs", AccessPermission::EventsBatch));
        assert!(!access.access.can_access_entity("standalone", AccessPermission::EventsBatch));
        app.api_keys.expire_for_seed(&key.id).unwrap();
        assert!(app.api_keys.authenticate(&plaintext).unwrap().is_none());
        let (_, replacement) = app.api_keys.regenerate(&key.id, ApiKeyExpiration::Never).unwrap().unwrap();
        assert!(app.api_keys.authenticate(&replacement).unwrap().is_some());
        assert!(app.api_keys.all().unwrap()[0].expires_at.is_none());
        app.api_keys
            .update(
                &key.id,
                "selected project",
                &AccessScope::Selected(vec![]),
                &AccessScope::Selected(vec!["site".into()]),
                &[AccessPermission::EventsBatch],
            )
            .unwrap();
        app.projects.delete("site").unwrap();
        assert!(
            !app.api_keys
                .authenticate(&replacement)
                .unwrap()
                .unwrap()
                .access
                .can_access_entity("docs", AccessPermission::EventsBatch)
        );
    }
}
