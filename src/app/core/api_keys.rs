use std::collections::HashSet;

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use rand::RngExt;
use rusqlite::{OptionalExtension, Transaction};

use crate::app::{
    SqlitePool,
    models::{ApiKey, ApiPermission},
};

const KEY_PREFIX: &str = "liw_";

/// The permissions and entities assigned to an authenticated API key.
pub struct ApiKeyAccess {
    pub id: String,
    entities: HashSet<String>,
    permissions: HashSet<ApiPermission>,
}

impl ApiKeyAccess {
    pub fn can_write_events(&self, entity_id: &str) -> bool {
        self.permissions.contains(&ApiPermission::EventsWrite) && self.entities.contains(entity_id)
    }
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
        entities: &[String],
        permissions: &[ApiPermission],
    ) -> Result<(ApiKey, String)> {
        let display_name = display_name.trim();
        validate_display_name(display_name)?;

        let secret = URL_SAFE_NO_PAD.encode(rand::rng().random::<[u8; 24]>());
        let plaintext = format!("{KEY_PREFIX}{secret}");
        let id = blake3::hash(secret.as_bytes()).to_hex().to_string();
        let permissions_json = serde_json::to_string(permissions)?;
        let created_at = Utc::now();
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute(
            "insert into api_keys (id, display_name, permissions_json, created_at) values (?, ?, ?, ?)",
            rusqlite::params![id, display_name, permissions_json, created_at],
        )?;
        set_access(&tx, &id, entities, permissions)?;
        tx.commit()?;

        Ok((
            ApiKey {
                id,
                display_name: display_name.to_string(),
                entities: entities.to_vec(),
                permissions: permissions.to_vec(),
                created_at,
                last_used_at: None,
                revoked_at: None,
            },
            plaintext,
        ))
    }

    /// Lists all key metadata without exposing hashes.
    pub fn all(&self) -> Result<Vec<ApiKey>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "select id, display_name, created_at, revoked_at, last_used_at,
                (select json_group_array(entity_id) from api_key_entities where key_id = api_keys.id),
                permissions_json
             from api_keys order by created_at desc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut keys = Vec::new();
        for row in rows {
            let (id, display_name, created_at, revoked_at, last_used_at, entities, permissions) = row?;
            keys.push(ApiKey {
                id,
                display_name,
                entities: serde_json::from_str(&entities)?,
                permissions: serde_json::from_str(&permissions)?,
                created_at,
                last_used_at,
                revoked_at,
            });
        }
        Ok(keys)
    }

    /// Updates a key's display name, entities, and permissions.
    pub fn update(
        &self,
        key_id: &str,
        display_name: &str,
        entities: &[String],
        permissions: &[ApiPermission],
    ) -> Result<bool> {
        let display_name = display_name.trim();
        validate_display_name(display_name)?;
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        if !tx.prepare_cached("select 1 from api_keys where id = ? and revoked_at is null limit 1")?.exists([key_id])? {
            return Ok(false);
        }
        tx.execute("update api_keys set display_name = ? where id = ?", [display_name, key_id])?;
        set_access(&tx, key_id, entities, permissions)?;
        tx.commit()?;
        Ok(true)
    }

    /// Revokes a key. Returns false when the ID is unknown.
    pub fn revoke(&self, key_id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        Ok(conn.execute(
            "update api_keys set revoked_at = coalesce(revoked_at, ?) where id = ?",
            rusqlite::params![Utc::now(), key_id],
        )? > 0)
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
                    permissions_json
                 from api_keys where id = ? and revoked_at is null",
                [id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()?;
        let Some((id, entities, permissions)) = access else { return Ok(None) };
        conn.execute("update api_keys set last_used_at = ? where id = ?", rusqlite::params![Utc::now(), id])?;
        Ok(Some(ApiKeyAccess {
            id,
            entities: serde_json::from_str(&entities)?,
            permissions: serde_json::from_str(&permissions)?,
        }))
    }
}

fn validate_display_name(display_name: &str) -> Result<()> {
    if display_name.is_empty() || display_name.len() > 100 {
        bail!("API key name must be between 1 and 100 characters");
    }
    Ok(())
}

fn set_access(tx: &Transaction<'_>, key_id: &str, entities: &[String], permissions: &[ApiPermission]) -> Result<()> {
    tx.execute("delete from api_key_entities where key_id = ?", [key_id])?;
    let mut entity_exists = tx.prepare_cached("select 1 from entities where id = ? limit 1")?;
    for entity_id in entities {
        if !entity_exists.exists([entity_id])? {
            bail!("entity not found: {entity_id}");
        }
        tx.execute("insert or ignore into api_key_entities (key_id, entity_id) values (?, ?)", [key_id, entity_id])?;
    }
    tx.execute(
        "update api_keys set permissions_json = ? where id = ?",
        rusqlite::params![serde_json::to_string(permissions)?, key_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{
            Liwan,
            models::{ApiPermission, Entity},
        },
        config::Config,
    };

    #[test]
    fn key_access_and_revocation_apply_immediately() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        app.entities.create(&Entity { id: "docs".into(), display_name: "Docs".into() }, &[]).unwrap();
        app.entities.create(&Entity { id: "shop".into(), display_name: "Shop".into() }, &[]).unwrap();
        let (key, plaintext) =
            app.api_keys.create("production", &["docs".into()], &[ApiPermission::EventsWrite]).unwrap();

        let access = app.api_keys.authenticate(&plaintext).unwrap().unwrap();
        assert_eq!(access.id, key.id);
        assert!(app.api_keys.all().unwrap()[0].last_used_at.is_some());
        assert!(access.can_write_events("docs"));
        assert!(!access.can_write_events("shop"));
        app.entities.delete("docs").unwrap();
        assert!(!app.api_keys.authenticate(&plaintext).unwrap().unwrap().can_write_events("docs"));
        app.api_keys.update(&key.id, "Production", &["shop".into()], &[ApiPermission::EventsWrite]).unwrap();
        assert_eq!(app.api_keys.all().unwrap()[0].display_name, "Production");
        assert!(app.api_keys.authenticate(&plaintext).unwrap().unwrap().can_write_events("shop"));
        app.api_keys.update(&key.id, "Production", &["shop".into()], &[]).unwrap();
        assert!(!app.api_keys.authenticate(&plaintext).unwrap().unwrap().can_write_events("shop"));
        assert!(app.api_keys.revoke(&key.id).unwrap());
        assert!(app.api_keys.authenticate(&plaintext).unwrap().is_none());
    }
}
