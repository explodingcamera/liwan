use anyhow::{Result, bail};
use sqlx::{Row, sqlite::SqliteRow};

use crate::app::{SqlitePool, models};
use crate::utils::validate;

#[derive(Clone)]
pub struct LiwanProjects {
    pool: SqlitePool,
}

fn to_project(row: &SqliteRow) -> Result<models::Project, sqlx::Error> {
    Ok(models::Project {
        id: row.try_get("id")?,
        display_name: row.try_get("display_name")?,
        public: row.try_get("public")?,
        unlisted: row.try_get("unlisted")?,
        secret: row.try_get("secret")?,
    })
}

impl LiwanProjects {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Replace the entities associated with a project
    pub async fn update_entities(&self, project_id: &str, entity_ids: &[String]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("delete from project_entities where project_id = ?").bind(project_id).execute(&mut *tx).await?;
        for entity_id in entity_ids {
            sqlx::query("insert into project_entities (project_id, entity_id) values (?, ?)")
                .bind(project_id)
                .bind(entity_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Get all entities associated with a project
    pub async fn entities(&self, project_id: &str) -> Result<Vec<models::Entity>> {
        let rows = sqlx::query(
            "select e.id, e.display_name from entities e join project_entities pe on e.id = pe.entity_id where pe.project_id = ?",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| Ok(models::Entity { id: row.try_get("id")?, display_name: row.try_get("display_name")? }))
            .collect::<Result<Vec<models::Entity>, sqlx::Error>>()?)
    }

    /// Get all entity IDs associated with a project
    pub async fn entity_ids(&self, project_id: &str) -> Result<Vec<String>> {
        let entities = sqlx::query_scalar("select entity_id from project_entities where project_id = ?")
            .bind(project_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(entities)
    }

    /// Get a project by ID
    pub async fn get(&self, id: &str) -> Result<models::Project> {
        let row = sqlx::query("select id, display_name, public, unlisted, secret from projects where id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(to_project(&row)?)
    }

    /// Get all projects
    pub async fn all(&self) -> Result<Vec<models::Project>> {
        let rows = sqlx::query("select id, display_name, public, unlisted, secret from projects")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(to_project).collect::<Result<Vec<models::Project>, sqlx::Error>>()?)
    }

    /// Create a new project
    pub async fn create(&self, project: &models::Project, initial_entities: &[String]) -> Result<models::Project> {
        if !validate::is_valid_id(&project.id) {
            bail!("invalid project ID");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("insert into projects (id, display_name, public, unlisted, secret) values (?, ?, ?, ?, ?)")
            .bind(&project.id)
            .bind(&project.display_name)
            .bind(project.public)
            .bind(project.unlisted)
            .bind(&project.secret)
            .execute(&mut *tx)
            .await?;
        for entity_id in initial_entities {
            sqlx::query("insert into project_entities (project_id, entity_id) values (?, ?)")
                .bind(&project.id)
                .bind(entity_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(project.clone())
    }

    /// Update a project
    pub async fn update(&self, project: &models::Project) -> Result<models::Project> {
        sqlx::query("update projects set display_name = ?, public = ?, unlisted = ?, secret = ? where id = ?")
            .bind(&project.display_name)
            .bind(project.public)
            .bind(project.unlisted)
            .bind(&project.secret)
            .bind(&project.id)
            .execute(&self.pool)
            .await?;
        Ok(project.clone())
    }

    /// Delete a project and its entity memberships
    pub async fn delete(&self, id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("delete from project_settings where project_id = ?").bind(id).execute(&mut *tx).await?;
        sqlx::query("delete from projects where id = ?").bind(id).execute(&mut *tx).await?;
        sqlx::query("delete from project_entities where project_id = ?").bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
