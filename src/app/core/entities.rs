use anyhow::{Result, bail};
use quick_cache::sync::Cache;
use std::sync::Arc;

use crate::app::{SqlitePool, models};
use crate::utils::validate;

#[derive(Clone)]
pub struct LiwanEntities {
    pool: SqlitePool,
    existing: Arc<Cache<String, ()>>,
}

impl LiwanEntities {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool, existing: Arc::new(Cache::new(512)) }
    }

    /// Get all entities
    pub fn all(&self) -> Result<Vec<models::Entity>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select id, display_name from entities")?;
        let entities = stmt
            .query_map([], |row| Ok(models::Entity { id: row.get("id")?, display_name: row.get("display_name")? }))?;
        Ok(entities.collect::<Result<Vec<models::Entity>, rusqlite::Error>>()?)
    }

    /// Create a new entity
    pub fn create(&self, entity: &models::Entity, initial_projects: &[String]) -> Result<()> {
        if !validate::is_valid_id(&entity.id) {
            bail!("invalid entity ID");
        }

        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute(
            "insert into entities (id, display_name) values (:id, :display_name)",
            rusqlite::named_params! { ":id": entity.id, ":display_name": entity.display_name },
        )?;
        {
            let mut exists = tx.prepare_cached("select 1 from projects where id = ? limit 1")?;
            for project_id in initial_projects {
                if !exists.exists([project_id])? {
                    bail!("project not found: {project_id}");
                }
                tx.execute(
                    "insert into project_entities (project_id, entity_id) values (:project_id, :entity_id)",
                    rusqlite::named_params! { ":project_id": project_id, ":entity_id": entity.id },
                )?;
            }
        }
        tx.commit()?;
        self.existing.insert(entity.id.clone(), ());
        Ok(())
    }

    /// Update an entity
    pub fn update(&self, entity: &models::Entity) -> Result<models::Entity> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update entities set display_name = :display_name where id = :id")?;
        stmt.execute(rusqlite::named_params! { ":display_name": entity.display_name, ":id": entity.id })?;
        Ok(entity.clone())
    }

    /// Update an entity's project memberships
    pub fn update_projects(&self, entity_id: &str, project_ids: &[String]) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from project_entities where entity_id = ?", rusqlite::params![entity_id])?;
        {
            let mut exists = tx.prepare_cached("select 1 from projects where id = ? limit 1")?;
            for project_id in project_ids {
                if !exists.exists([project_id])? {
                    bail!("project not found: {project_id}");
                }
                tx.execute(
                    "insert into project_entities (project_id, entity_id) values (:project_id, :entity_id)",
                    rusqlite::named_params! { ":project_id": project_id, ":entity_id": entity_id },
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete an entity without removing associated events
    pub fn delete(&self, id: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from entity_settings where entity_id = ?", rusqlite::params![id])?;
        tx.execute("delete from entities where id = ?", rusqlite::params![id])?;
        tx.execute("delete from project_entities where entity_id = ?", rusqlite::params![id])?;
        tx.commit()?;
        self.existing.remove(id);
        Ok(())
    }

    /// Get all projects associated with an entity
    pub fn projects(&self, entity_id: &str) -> Result<Vec<models::Project>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "select p.id, p.display_name, p.public, p.unlisted, p.secret from projects p join project_entities pe on p.id = pe.project_id where pe.entity_id = ?",
        )?;
        let projects = stmt.query_map(rusqlite::params![entity_id], |row| {
            Ok(models::Project {
                id: row.get("id")?,
                display_name: row.get("display_name")?,
                public: row.get("public")?,
                unlisted: row.get("unlisted")?,
                secret: row.get("secret")?,
            })
        })?;
        Ok(projects.collect::<Result<Vec<models::Project>, rusqlite::Error>>()?)
    }

    /// Check if an entity exists
    pub fn exists(&self, id: &str) -> Result<bool> {
        if self.existing.get(id).is_some() {
            return Ok(true);
        }

        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("select 1 from entities where id = ? limit 1")?;
        let exists = stmt.exists([id])?;
        if exists {
            self.existing.insert(id.to_string(), ());
        }
        Ok(exists)
    }
}
