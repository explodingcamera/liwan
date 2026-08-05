use anyhow::{Result, bail};
use sqlx::Row;

use crate::app::{SqlitePool, models};
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
    pub async fn all(&self) -> Result<Vec<models::Entity>> {
        let rows = sqlx::query("select id, display_name from entities").fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| Ok(models::Entity { id: row.try_get("id")?, display_name: row.try_get("display_name")? }))
            .collect::<Result<Vec<models::Entity>, sqlx::Error>>()?)
    }

    /// Create a new entity
    pub async fn create(&self, entity: &models::Entity, initial_projects: &[String]) -> Result<()> {
        if !validate::is_valid_id(&entity.id) {
            bail!("invalid entity ID");
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("insert into entities (id, display_name) values (?, ?)")
            .bind(&entity.id)
            .bind(&entity.display_name)
            .execute(&mut *tx)
            .await?;
        for project_id in initial_projects {
            sqlx::query("insert into project_entities (project_id, entity_id) values (?, ?)")
                .bind(project_id)
                .bind(&entity.id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Update an entity
    pub async fn update(&self, entity: &models::Entity) -> Result<models::Entity> {
        sqlx::query("update entities set display_name = ? where id = ?")
            .bind(&entity.display_name)
            .bind(&entity.id)
            .execute(&self.pool)
            .await?;
        Ok(entity.clone())
    }

    /// Update an entity's project memberships
    pub async fn update_projects(&self, entity_id: &str, project_ids: &[String]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("delete from project_entities where entity_id = ?").bind(entity_id).execute(&mut *tx).await?;
        for project_id in project_ids {
            sqlx::query("insert into project_entities (project_id, entity_id) values (?, ?)")
                .bind(project_id)
                .bind(entity_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Delete an entity without removing associated events
    pub async fn delete(&self, id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("delete from entity_settings where entity_id = ?").bind(id).execute(&mut *tx).await?;
        sqlx::query("delete from entities where id = ?").bind(id).execute(&mut *tx).await?;
        sqlx::query("delete from project_entities where entity_id = ?").bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Get all projects associated with an entity
    pub async fn projects(&self, entity_id: &str) -> Result<Vec<models::Project>> {
        let rows = sqlx::query(
            "select p.id, p.display_name, p.public, p.unlisted, p.secret from projects p join project_entities pe on p.id = pe.project_id where pe.entity_id = ?",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| {
                Ok(models::Project {
                    id: row.try_get("id")?,
                    display_name: row.try_get("display_name")?,
                    public: row.try_get("public")?,
                    unlisted: row.try_get("unlisted")?,
                    secret: row.try_get("secret")?,
                })
            })
            .collect::<Result<Vec<models::Project>, sqlx::Error>>()?)
    }

    /// Check if an entity exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let exists = sqlx::query("select 1 from entities where id = ? limit 1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();
        Ok(exists)
    }
}
