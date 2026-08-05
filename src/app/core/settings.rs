use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};

use anyhow::{Result, bail};
use sqlx::{Row, sqlite::SqliteRow};

use crate::app::{SqlitePool, models};
use crate::utils::sqlite::decode_err;

#[derive(Clone)]
pub struct LiwanSettings {
    pool: SqlitePool,
    cache: Arc<RwLock<SettingsCache>>,
}

#[derive(Clone)]
pub struct LiwanProjectSettings {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
struct SettingsCache {
    global: models::CollectionSettings,
    entities: HashMap<String, models::EntityCollectionSettings>,
}

impl LiwanSettings {
    pub async fn try_new(pool: SqlitePool) -> Result<Self> {
        let cache = SettingsCache::load(&pool).await?;
        Ok(Self { pool, cache: Arc::new(RwLock::new(cache)) })
    }

    /// Get the global collection settings
    pub fn global(&self) -> models::CollectionSettings {
        self.cache.read().expect("collection settings cache poisoned").global.clone()
    }

    /// Get the per-entity settings, returning inherit defaults when absent
    pub fn entity(&self, entity_id: &str) -> models::EntityCollectionSettings {
        self.cache.read().expect("collection settings cache poisoned").entities.get(entity_id).cloned().unwrap_or_else(
            || models::EntityCollectionSettings {
                entity_id: entity_id.to_string(),
                visitor_group_mode: None,
                track_sessions: None,
                track_utm_params: None,
                track_geo: None,
                data_retention: models::DataRetention::Inherit,
                allowed_hostnames: Vec::new(),
                ingest_drop_rules: Vec::new(),
            },
        )
    }

    /// Resolve global and entity settings into the effective collection settings
    pub fn resolved_for_entity(&self, entity_id: &str) -> models::ResolvedCollectionSettings {
        let cache = self.cache.read().expect("collection settings cache poisoned");
        models::ResolvedCollectionSettings::resolve(cache.global.clone(), cache.entities.get(entity_id).cloned())
    }

    /// Update global collection settings and refresh the cache
    pub async fn update_global(&self, settings: &models::CollectionSettings) -> Result<()> {
        if settings.data_retention == models::DataRetention::Inherit {
            bail!("global data_retention cannot inherit");
        }

        let ingest_drop_rules_json = serde_json::to_string(&settings.ingest_drop_rules)?;
        let data_retention_days = match settings.data_retention {
            models::DataRetention::All => None,
            models::DataRetention::Days(days) => Some(days.get()),
            models::DataRetention::Inherit => unreachable!(),
        };
        sqlx::query(
            "update settings
             set
                visitor_group_mode = ?,
                track_sessions = ?,
                track_utm_params = ?,
                track_geo = ?,
                history_days = ?,
                ingest_drop_rules_json = ?
             where id = 1",
        )
        .bind(settings.visitor_group_mode.to_string())
        .bind(settings.track_sessions)
        .bind(settings.track_utm_params)
        .bind(settings.track_geo.to_string())
        .bind(data_retention_days)
        .bind(ingest_drop_rules_json)
        .execute(&self.pool)
        .await?;
        self.reload().await?;
        Ok(())
    }

    /// Update per-entity collection settings and refresh the cache
    pub async fn update_entity(&self, settings: &models::EntityCollectionSettings) -> Result<()> {
        let mut allowed_hostnames = Vec::new();
        for pattern in &settings.allowed_hostnames {
            if let Some(pattern) = models::normalize_allowed_hostname_pattern(pattern).map_err(anyhow::Error::msg)?
                && !allowed_hostnames.contains(&pattern)
            {
                allowed_hostnames.push(pattern);
            }
        }
        let allowed_hostnames = allowed_hostnames.join(",");
        let ingest_drop_rules_json = serde_json::to_string(&settings.ingest_drop_rules)?;
        let history_mode = match settings.data_retention {
            models::DataRetention::Inherit => "inherit",
            models::DataRetention::All => "keep_all",
            models::DataRetention::Days(_) => "days",
        };
        let data_retention_days = match settings.data_retention {
            models::DataRetention::Days(days) => Some(days.get()),
            models::DataRetention::Inherit | models::DataRetention::All => None,
        };
        sqlx::query(
            "insert into entity_settings (entity_id, visitor_group_mode, track_sessions, track_utm_params, track_geo, history_mode, history_days, allowed_hostnames, ingest_drop_rules_json)
             values (?, ?, ?, ?, ?, ?, ?, ?, ?)
             on conflict(entity_id) do update set
                visitor_group_mode = excluded.visitor_group_mode,
                track_sessions = excluded.track_sessions,
                track_utm_params = excluded.track_utm_params,
                track_geo = excluded.track_geo,
                history_mode = excluded.history_mode,
                history_days = excluded.history_days,
                allowed_hostnames = excluded.allowed_hostnames,
                ingest_drop_rules_json = excluded.ingest_drop_rules_json",
        )
        .bind(&settings.entity_id)
        .bind(settings.visitor_group_mode.map(|mode| mode.to_string()))
        .bind(settings.track_sessions)
        .bind(settings.track_utm_params)
        .bind(settings.track_geo.map(|detail| detail.to_string()))
        .bind(history_mode)
        .bind(data_retention_days)
        .bind(allowed_hostnames)
        .bind(ingest_drop_rules_json)
        .execute(&self.pool)
        .await?;
        self.reload().await?;
        Ok(())
    }

    /// Reload collection settings from SQLite into the in-memory cache
    pub async fn reload(&self) -> Result<()> {
        let cache = SettingsCache::load(&self.pool).await?;
        *self.cache.write().expect("collection settings cache poisoned") = cache;
        Ok(())
    }
}

impl LiwanProjectSettings {
    /// Create a project settings store backed by SQLite
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get display settings for a project
    pub async fn get(&self, project_id: &str) -> Result<models::ProjectDisplaySettings> {
        let row = sqlx::query(
            "select metric_display_overrides_json, dimension_display_overrides_json from project_settings where project_id = ?",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        let settings = row
            .map(|row| {
                let metric_json: String = row.try_get(0)?;
                let dimension_json: String = row.try_get(1)?;
                Ok::<_, sqlx::Error>(models::ProjectDisplaySettings {
                    project_id: project_id.to_string(),
                    metric_display_overrides: serde_json::from_str(&metric_json)
                        .map_err(|err| decode_err("metric_display_overrides_json", err))?,
                    dimension_display_overrides: serde_json::from_str(&dimension_json)
                        .map_err(|err| decode_err("dimension_display_overrides_json", err))?,
                })
            })
            .transpose()?;

        Ok(settings.unwrap_or_else(|| models::ProjectDisplaySettings {
            project_id: project_id.to_string(),
            ..Default::default()
        }))
    }

    /// Update display settings for a project
    pub async fn update(&self, settings: &models::ProjectDisplaySettings) -> Result<()> {
        let metric_json = serde_json::to_string(&settings.metric_display_overrides)?;
        let dimension_json = serde_json::to_string(&settings.dimension_display_overrides)?;
        sqlx::query(
            "insert into project_settings (project_id, metric_display_overrides_json, dimension_display_overrides_json)
             values (?, ?, ?)
             on conflict(project_id) do update set
                metric_display_overrides_json = excluded.metric_display_overrides_json,
                dimension_display_overrides_json = excluded.dimension_display_overrides_json",
        )
        .bind(&settings.project_id)
        .bind(metric_json)
        .bind(dimension_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl SettingsCache {
    async fn load(pool: &SqlitePool) -> Result<Self> {
        let global = sqlx::query(
            "select visitor_group_mode, track_sessions, track_utm_params, track_geo, history_days, ingest_drop_rules_json from settings where id = 1",
        )
        .fetch_one(pool)
        .await
        .and_then(|row| {
            let visitor_group_mode: String = row.try_get(0)?;
            let track_geo: String = row.try_get(3)?;
            let history_days: Option<u32> = row.try_get(4)?;
            let ingest_drop_rules_json: String = row.try_get(5)?;
            let data_retention = match history_days {
                Some(days) => models::DataRetention::Days(NonZeroU32::new(days).ok_or_else(|| {
                    decode_err("history_days", "data retention days must be greater than zero")
                })?),
                None => models::DataRetention::All,
            };

            Ok(models::CollectionSettings {
                visitor_group_mode: visitor_group_mode
                    .parse()
                    .map_err(|err: String| decode_err("visitor_group_mode", err))?,
                track_sessions: row.try_get(1)?,
                track_utm_params: row.try_get(2)?,
                track_geo: track_geo.parse().map_err(|err: String| decode_err("track_geo", err))?,
                data_retention,
                ingest_drop_rules: serde_json::from_str(&ingest_drop_rules_json)
                    .map_err(|err| decode_err("ingest_drop_rules_json", err))?,
            })
        })?;

        let rows = sqlx::query(
            "select entity_id, visitor_group_mode, track_sessions, track_utm_params, track_geo, history_mode, history_days, allowed_hostnames, ingest_drop_rules_json from entity_settings",
        )
        .fetch_all(pool)
        .await?;

        let entities = rows
            .iter()
            .map(to_entity_settings)
            .collect::<Result<Vec<_>, sqlx::Error>>()?
            .into_iter()
            .map(|settings| (settings.entity_id.clone(), settings))
            .collect();

        Ok(Self { global, entities })
    }
}

fn to_entity_settings(row: &SqliteRow) -> Result<models::EntityCollectionSettings, sqlx::Error> {
    let visitor_group_mode: Option<String> = row.try_get(1)?;
    let track_geo: Option<String> = row.try_get(4)?;
    let history_mode: String = row.try_get(5)?;
    let history_days: Option<u32> = row.try_get(6)?;
    let allowed_hostnames: String = row.try_get(7)?;
    let ingest_drop_rules_json: String = row.try_get(8)?;
    let data_retention = match history_mode.as_str() {
        "inherit" => models::DataRetention::Inherit,
        "keep_all" => models::DataRetention::All,
        "days" => models::DataRetention::Days(
            history_days
                .and_then(NonZeroU32::new)
                .ok_or_else(|| decode_err("history_days", "data retention days must be greater than zero"))?,
        ),
        _ => {
            return Err(decode_err("history_mode", format!("invalid history mode: {history_mode}")));
        }
    };

    Ok(models::EntityCollectionSettings {
        entity_id: row.try_get(0)?,
        visitor_group_mode: visitor_group_mode
            .map(|value| value.parse().map_err(|err: String| decode_err("visitor_group_mode", err)))
            .transpose()?,
        track_sessions: row.try_get(2)?,
        track_utm_params: row.try_get(3)?,
        track_geo: track_geo
            .map(|value| value.parse().map_err(|err: String| decode_err("track_geo", err)))
            .transpose()?,
        data_retention,
        allowed_hostnames: allowed_hostnames
            .split(',')
            .filter_map(|pattern| models::normalize_allowed_hostname_pattern(pattern).ok().flatten())
            .collect(),
        ingest_drop_rules: serde_json::from_str(&ingest_drop_rules_json)
            .map_err(|err| decode_err("ingest_drop_rules_json", err))?,
    })
}
