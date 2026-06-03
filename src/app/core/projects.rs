use anyhow::{Result, bail};

use crate::app::{SqlitePool, models};
use crate::utils::validate;

#[derive(Clone)]
pub struct LiwanProjects {
    pool: SqlitePool,
}

impl LiwanProjects {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Replace the entities associated with a project
    pub fn update_entities(&self, project_id: &str, entity_ids: &[String]) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from project_entities where project_id = ?", rusqlite::params![project_id])?;
        for entity_id in entity_ids {
            tx.execute(
                "insert into project_entities (project_id, entity_id) values (:project_id, :entity_id)",
                rusqlite::named_params! { ":project_id": project_id, ":entity_id": entity_id },
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Get all entities associated with a project
    pub fn entities(&self, project_id: &str) -> Result<Vec<models::Entity>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "select e.id, e.display_name from entities e join project_entities pe on e.id = pe.entity_id where pe.project_id = ?",
        )?;
        let entities = stmt.query_map(rusqlite::params![project_id], |row| {
            Ok(models::Entity { id: row.get("id")?, display_name: row.get("display_name")? })
        })?;
        Ok(entities.collect::<Result<Vec<models::Entity>, rusqlite::Error>>()?)
    }

    /// Get all entity IDs associated with a project
    pub fn entity_ids(&self, project_id: &str) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("select entity_id from project_entities where project_id = ?")?;
        let entities = stmt.query_map(rusqlite::params![project_id], |row| row.get("entity_id"))?;
        Ok(entities.collect::<Result<Vec<String>, rusqlite::Error>>()?)
    }

    /// Get a project by ID
    pub fn get(&self, id: &str) -> Result<models::Project> {
        let conn = self.pool.get()?;
        let project = conn.prepare("select id, display_name, public, secret from projects where id = ?")?.query_row(
            rusqlite::params![id],
            |row| {
                Ok(models::Project {
                    id: row.get("id")?,
                    display_name: row.get("display_name")?,
                    public: row.get("public")?,
                    secret: row.get("secret")?,
                })
            },
        )?;
        Ok(project)
    }

    /// Get all projects
    pub fn all(&self) -> Result<Vec<models::Project>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("select id, display_name, public, secret from projects")?;
        let projects = stmt.query_map([], |row| {
            Ok(models::Project {
                id: row.get("id")?,
                display_name: row.get("display_name")?,
                public: row.get("public")?,
                secret: row.get("secret")?,
            })
        })?;

        Ok(projects.collect::<Result<Vec<models::Project>, rusqlite::Error>>()?)
    }

    /// Create a new project
    pub fn create(&self, project: &models::Project, initial_entities: &[String]) -> Result<models::Project> {
        if !validate::is_valid_id(&project.id) {
            bail!("invalid project ID");
        }
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute(
            "insert into projects (id, display_name, public, secret) values (:id, :display_name, :public, :secret)",
            rusqlite::named_params! {
                ":id": project.id,
                ":display_name": project.display_name,
                ":public": project.public,
                ":secret": project.secret,
            },
        )?;
        for entity_id in initial_entities {
            tx.execute(
                "insert into project_entities (project_id, entity_id) values (:project_id, :entity_id)",
                rusqlite::named_params! { ":project_id": project.id, ":entity_id": entity_id },
            )?;
        }
        tx.commit()?;
        Ok(project.clone())
    }

    /// Update a project
    pub fn update(&self, project: &models::Project) -> Result<models::Project> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "update projects set display_name = :display_name, public = :public, secret = :secret where id = :id",
        )?;
        stmt.execute(rusqlite::named_params! {
            ":display_name": project.display_name,
            ":public": project.public,
            ":secret": project.secret,
            ":id": project.id,
        })?;
        Ok(project.clone())
    }

    /// Delete a project and its entity memberships
    pub fn delete(&self, id: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from project_settings where project_id = ?", rusqlite::params![id])?;
        tx.execute("delete from projects where id = ?", rusqlite::params![id])?;
        tx.execute("delete from project_entities where project_id = ?", rusqlite::params![id])?;
        tx.commit()?;
        Ok(())
    }
}
