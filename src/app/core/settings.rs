use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, RwLock};

use anyhow::{Result, bail};
use rusqlite::OptionalExtension;

use crate::app::{SqlitePool, models};

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
    pub fn try_new(pool: SqlitePool) -> Result<Self> {
        let cache = SettingsCache::load(&pool)?;
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
    pub fn update_global(&self, settings: &models::CollectionSettings) -> Result<()> {
        if settings.data_retention == models::DataRetention::Inherit {
            bail!("global data_retention cannot inherit");
        }

        let ingest_drop_rules_json = serde_json::to_string(&settings.ingest_drop_rules)?;
        let data_retention_days = match settings.data_retention {
            models::DataRetention::All => None,
            models::DataRetention::Days(days) => Some(days.get()),
            models::DataRetention::Inherit => unreachable!(),
        };
        let conn = self.pool.get()?;
        conn.execute(
            "update settings
             set
                visitor_group_mode = :visitor_group_mode,
                track_sessions = :track_sessions,
                track_utm_params = :track_utm_params,
                track_geo = :track_geo,
                history_days = :history_days,
                ingest_drop_rules_json = :ingest_drop_rules_json
             where id = 1",
            rusqlite::named_params! {
                ":visitor_group_mode": settings.visitor_group_mode.to_string(),
                ":track_sessions": settings.track_sessions,
                ":track_utm_params": settings.track_utm_params,
                ":track_geo": settings.track_geo.to_string(),
                ":history_days": data_retention_days,
                ":ingest_drop_rules_json": ingest_drop_rules_json,
            },
        )?;
        self.reload()?;
        Ok(())
    }

    /// Update per-entity collection settings and refresh the cache
    pub fn update_entity(&self, settings: &models::EntityCollectionSettings) -> Result<()> {
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
        let conn = self.pool.get()?;
        conn.execute(
            "insert into entity_settings (entity_id, visitor_group_mode, track_sessions, track_utm_params, track_geo, history_mode, history_days, allowed_hostnames, ingest_drop_rules_json)
             values (:entity_id, :visitor_group_mode, :track_sessions, :track_utm_params, :track_geo, :history_mode, :history_days, :allowed_hostnames, :ingest_drop_rules_json)
             on conflict(entity_id) do update set
                visitor_group_mode = excluded.visitor_group_mode,
                track_sessions = excluded.track_sessions,
                track_utm_params = excluded.track_utm_params,
                track_geo = excluded.track_geo,
                history_mode = excluded.history_mode,
                history_days = excluded.history_days,
                allowed_hostnames = excluded.allowed_hostnames,
                ingest_drop_rules_json = excluded.ingest_drop_rules_json",
            rusqlite::named_params! {
                ":entity_id": settings.entity_id,
                ":visitor_group_mode": settings.visitor_group_mode.map(|mode| mode.to_string()),
                ":track_sessions": settings.track_sessions,
                ":track_utm_params": settings.track_utm_params,
                ":track_geo": settings.track_geo.map(|detail| detail.to_string()),
                ":history_mode": history_mode,
                ":history_days": data_retention_days,
                ":allowed_hostnames": allowed_hostnames,
                ":ingest_drop_rules_json": ingest_drop_rules_json,
            },
        )?;
        self.reload()?;
        Ok(())
    }

    /// Reload collection settings from SQLite into the in-memory cache
    pub fn reload(&self) -> Result<()> {
        let cache = SettingsCache::load(&self.pool)?;
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
    pub fn get(&self, project_id: &str) -> Result<models::ProjectDisplaySettings> {
        let conn = self.pool.get()?;
        let settings = conn
            .query_row(
                "select metric_display_overrides_json, dimension_display_overrides_json from project_settings where project_id = ?",
                [project_id],
                |row| {
                    let metric_json: String = row.get(0)?;
                    let dimension_json: String = row.get(1)?;
                    Ok(models::ProjectDisplaySettings {
                        project_id: project_id.to_string(),
                        metric_display_overrides: serde_json::from_str(&metric_json)
                            .map_err(|err| sql_err(0, rusqlite::types::Type::Text, err))?,
                        dimension_display_overrides: serde_json::from_str(&dimension_json)
                            .map_err(|err| sql_err(1, rusqlite::types::Type::Text, err))?,
                    })
                },
            )
            .optional()?;

        Ok(settings.unwrap_or_else(|| models::ProjectDisplaySettings {
            project_id: project_id.to_string(),
            ..Default::default()
        }))
    }

    /// Update display settings for a project
    pub fn update(&self, settings: &models::ProjectDisplaySettings) -> Result<()> {
        let metric_json = serde_json::to_string(&settings.metric_display_overrides)?;
        let dimension_json = serde_json::to_string(&settings.dimension_display_overrides)?;
        let conn = self.pool.get()?;
        conn.execute(
            "insert into project_settings (project_id, metric_display_overrides_json, dimension_display_overrides_json)
             values (:project_id, :metric_display_overrides_json, :dimension_display_overrides_json)
             on conflict(project_id) do update set
                metric_display_overrides_json = excluded.metric_display_overrides_json,
                dimension_display_overrides_json = excluded.dimension_display_overrides_json",
            rusqlite::named_params! {
                ":project_id": settings.project_id,
                ":metric_display_overrides_json": metric_json,
                ":dimension_display_overrides_json": dimension_json,
            },
        )?;
        Ok(())
    }
}

impl SettingsCache {
    fn load(pool: &SqlitePool) -> Result<Self> {
        let conn = pool.get()?;
        let global = conn.query_row(
            "select visitor_group_mode, track_sessions, track_utm_params, track_geo, history_days, ingest_drop_rules_json from settings where id = 1",
            [],
            |row| {
                let visitor_group_mode: String = row.get(0)?;
                let track_geo: String = row.get(3)?;
                let history_days: Option<u32> = row.get(4)?;
                let ingest_drop_rules_json: String = row.get(5)?;
                let data_retention = match history_days {
                    Some(days) => models::DataRetention::Days(
                        NonZeroU32::new(days).ok_or_else(|| {
                            sql_err(4, rusqlite::types::Type::Integer, "data retention days must be greater than zero")
                        })?,
                    ),
                    None => models::DataRetention::All,
                };

                Ok(models::CollectionSettings {
                    visitor_group_mode: visitor_group_mode
                        .parse()
                        .map_err(|err: String| sql_err(0, rusqlite::types::Type::Text, err))?,
                    track_sessions: row.get(1)?,
                    track_utm_params: row.get(2)?,
                    track_geo: track_geo
                        .parse()
                        .map_err(|err: String| sql_err(3, rusqlite::types::Type::Text, err))?,
                    data_retention,
                    ingest_drop_rules: serde_json::from_str(&ingest_drop_rules_json)
                        .map_err(|err| sql_err(5, rusqlite::types::Type::Text, err))?,
                })
            },
        )?;

        let mut stmt = conn.prepare(
            "select entity_id, visitor_group_mode, track_sessions, track_utm_params, track_geo, history_mode, history_days, allowed_hostnames, ingest_drop_rules_json from entity_settings",
        )?;
        let entities = stmt
            .query_map([], |row| {
                let visitor_group_mode: Option<String> = row.get(1)?;
                let track_geo: Option<String> = row.get(4)?;
                let history_mode: String = row.get(5)?;
                let history_days: Option<u32> = row.get(6)?;
                let allowed_hostnames: String = row.get(7)?;
                let ingest_drop_rules_json: String = row.get(8)?;
                let data_retention = match history_mode.as_str() {
                    "inherit" => models::DataRetention::Inherit,
                    "keep_all" => models::DataRetention::All,
                    "days" => models::DataRetention::Days(history_days.and_then(NonZeroU32::new).ok_or_else(|| {
                        sql_err(6, rusqlite::types::Type::Integer, "data retention days must be greater than zero")
                    })?),
                    _ => {
                        return Err(sql_err(
                            5,
                            rusqlite::types::Type::Text,
                            format!("invalid history mode: {history_mode}"),
                        ));
                    }
                };

                Ok(models::EntityCollectionSettings {
                    entity_id: row.get(0)?,
                    visitor_group_mode: visitor_group_mode
                        .map(|value| value.parse().map_err(|err: String| sql_err(1, rusqlite::types::Type::Text, err)))
                        .transpose()?,
                    track_sessions: row.get(2)?,
                    track_utm_params: row.get(3)?,
                    track_geo: track_geo
                        .map(|value| value.parse().map_err(|err: String| sql_err(4, rusqlite::types::Type::Text, err)))
                        .transpose()?,
                    data_retention,
                    allowed_hostnames: allowed_hostnames
                        .split(',')
                        .filter_map(|pattern| models::normalize_allowed_hostname_pattern(pattern).ok().flatten())
                        .collect(),
                    ingest_drop_rules: serde_json::from_str(&ingest_drop_rules_json)
                        .map_err(|err| sql_err(8, rusqlite::types::Type::Text, err))?,
                })
            })?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?
            .into_iter()
            .map(|settings| (settings.entity_id.clone(), settings))
            .collect();

        Ok(Self { global, entities })
    }
}

fn sql_err(column: usize, kind: rusqlite::types::Type, err: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        kind,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())),
    )
}
