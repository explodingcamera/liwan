use eyre::{bail, Result};

use crate::app::{models, SqlitePool};
use crate::utils::validate;

#[derive(Clone)]
pub struct LiwanEntities {
    pool: SqlitePool,
}

impl LiwanEntities {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
            "insert into entities (id, display_name) values (?, ?)",
            rusqlite::params![entity.id, entity.display_name],
        )?;
        for project_id in initial_projects {
            tx.execute(
                "insert into project_entities (project_id, entity_id) values (?, ?)",
                rusqlite::params![project_id, entity.id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Update an entity
    pub fn update(&self, entity: &models::Entity) -> Result<models::Entity> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("update entities set display_name = ? where id = ?")?;
        stmt.execute(rusqlite::params![entity.display_name, entity.id])?;
        Ok(entity.clone())
    }

    /// Update an entity's projects
    pub fn update_projects(&self, entity_id: &str, project_ids: &[String]) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from project_entities where entity_id = ?", rusqlite::params![entity_id])?;
        for project_id in project_ids {
            tx.execute(
                "insert into project_entities (project_id, entity_id) values (?, ?)",
                rusqlite::params![project_id, entity_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete an entity (does not remove associated events)
    pub fn delete(&self, id: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from entities where id = ?", rusqlite::params![id])?;
        tx.execute("delete from project_entities where entity_id = ?", rusqlite::params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Get all projects associated with an entity
    pub fn projects(&self, entity_id: &str) -> Result<Vec<models::Project>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "select p.id, p.display_name, p.public, p.secret from projects p join project_entities pe on p.id = pe.project_id where pe.entity_id = ?",
        )?;
        let projects = stmt.query_map(rusqlite::params![entity_id], |row| {
            Ok(models::Project {
                id: row.get("id")?,
                display_name: row.get("display_name")?,
                public: row.get("public")?,
                secret: row.get("secret")?,
            })
        })?;
        Ok(projects.collect::<Result<Vec<models::Project>, rusqlite::Error>>()?)
    }

    /// Check if an entity exists
    pub fn exists(&self, id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("select 1 from entities where id = ? limit 1")?;
        Ok(stmt.exists([id])?)
    }
}
