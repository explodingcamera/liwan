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

    pub fn global(&self) -> models::CollectionSettings {
        self.cache.read().expect("collection settings cache poisoned").global.clone()
    }

    pub fn entity(&self, entity_id: &str) -> models::EntityCollectionSettings {
        self.cache.read().expect("collection settings cache poisoned").entities.get(entity_id).cloned().unwrap_or_else(
            || models::EntityCollectionSettings {
                entity_id: entity_id.to_string(),
                visitor_group_mode: None,
                track_sessions: None,
                track_utm_params: None,
                track_geo: None,
                data_retention: models::DataRetention::Inherit,
                ingest_drop_rules: Vec::new(),
            },
        )
    }

    pub fn resolved_for_entity(&self, entity_id: &str) -> models::ResolvedCollectionSettings {
        let cache = self.cache.read().expect("collection settings cache poisoned");
        models::ResolvedCollectionSettings::resolve(cache.global.clone(), cache.entities.get(entity_id).cloned())
    }

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

    pub fn update_entity(&self, settings: &models::EntityCollectionSettings) -> Result<()> {
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
            "insert into entity_settings (entity_id, visitor_group_mode, track_sessions, track_utm_params, track_geo, history_mode, history_days, ingest_drop_rules_json)
             values (:entity_id, :visitor_group_mode, :track_sessions, :track_utm_params, :track_geo, :history_mode, :history_days, :ingest_drop_rules_json)
             on conflict(entity_id) do update set
                visitor_group_mode = excluded.visitor_group_mode,
                track_sessions = excluded.track_sessions,
                track_utm_params = excluded.track_utm_params,
                track_geo = excluded.track_geo,
                history_mode = excluded.history_mode,
                history_days = excluded.history_days,
                ingest_drop_rules_json = excluded.ingest_drop_rules_json",
            rusqlite::named_params! {
                ":entity_id": settings.entity_id,
                ":visitor_group_mode": settings.visitor_group_mode.map(|mode| mode.to_string()),
                ":track_sessions": settings.track_sessions,
                ":track_utm_params": settings.track_utm_params,
                ":track_geo": settings.track_geo.map(|detail| detail.to_string()),
                ":history_mode": history_mode,
                ":history_days": data_retention_days,
                ":ingest_drop_rules_json": ingest_drop_rules_json,
            },
        )?;
        self.reload()?;
        Ok(())
    }

    fn reload(&self) -> Result<()> {
        let cache = SettingsCache::load(&self.pool)?;
        *self.cache.write().expect("collection settings cache poisoned") = cache;
        Ok(())
    }
}

impl LiwanProjectSettings {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

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
                        metric_display_overrides: serde_json::from_str(&metric_json).map_err(to_sql_err)?,
                        dimension_display_overrides: serde_json::from_str(&dimension_json).map_err(to_sql_err)?,
                    })
                },
            )
            .optional()?;

        Ok(settings.unwrap_or_else(|| models::ProjectDisplaySettings {
            project_id: project_id.to_string(),
            ..Default::default()
        }))
    }

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
            Self::read_global,
        )?;

        let mut stmt = conn.prepare(
            "select entity_id, visitor_group_mode, track_sessions, track_utm_params, track_geo, history_mode, history_days, ingest_drop_rules_json from entity_settings",
        )?;
        let entities = stmt
            .query_map([], Self::read_entity)?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?
            .into_iter()
            .map(|settings| (settings.entity_id.clone(), settings))
            .collect();

        Ok(Self { global, entities })
    }

    fn read_global(row: &rusqlite::Row<'_>) -> rusqlite::Result<models::CollectionSettings> {
        let visitor_group_mode: String = row.get(0)?;
        let track_geo: String = row.get(3)?;
        let history_days: Option<u32> = row.get(4)?;
        let ingest_drop_rules_json: String = row.get(5)?;
        let data_retention = match history_days {
            Some(days) => models::DataRetention::Days(
                NonZeroU32::new(days)
                    .ok_or_else(|| to_sql_msg("data retention days must be greater than zero".to_string()))?,
            ),
            None => models::DataRetention::All,
        };
        Ok(models::CollectionSettings {
            visitor_group_mode: parse_db(visitor_group_mode)?,
            track_sessions: row.get(1)?,
            track_utm_params: row.get(2)?,
            track_geo: parse_db(track_geo)?,
            data_retention,
            ingest_drop_rules: serde_json::from_str(&ingest_drop_rules_json).map_err(to_sql_err)?,
        })
    }

    fn read_entity(row: &rusqlite::Row<'_>) -> rusqlite::Result<models::EntityCollectionSettings> {
        let visitor_group_mode: Option<String> = row.get(1)?;
        let track_geo: Option<String> = row.get(4)?;
        let history_mode: String = row.get(5)?;
        let history_days: Option<u32> = row.get(6)?;
        let ingest_drop_rules_json: String = row.get(7)?;
        let data_retention = match history_mode.as_str() {
            "inherit" => models::DataRetention::Inherit,
            "keep_all" => models::DataRetention::All,
            "days" => models::DataRetention::Days(
                history_days
                    .and_then(NonZeroU32::new)
                    .ok_or_else(|| to_sql_msg("data retention days must be greater than zero".to_string()))?,
            ),
            _ => return Err(to_sql_msg(format!("invalid history mode: {history_mode}"))),
        };
        Ok(models::EntityCollectionSettings {
            entity_id: row.get(0)?,
            visitor_group_mode: visitor_group_mode.map(parse_db).transpose()?,
            track_sessions: row.get(2)?,
            track_utm_params: row.get(3)?,
            track_geo: track_geo.map(parse_db).transpose()?,
            data_retention,
            ingest_drop_rules: serde_json::from_str(&ingest_drop_rules_json).map_err(to_sql_err)?,
        })
    }
}

fn parse_db<T>(value: String) -> rusqlite::Result<T>
where
    T: std::str::FromStr<Err = String>,
{
    value.parse().map_err(to_sql_msg)
}

fn to_sql_err(err: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
}

fn to_sql_msg(err: String) -> rusqlite::Error {
    to_sql_err(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}
