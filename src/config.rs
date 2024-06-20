use eyre::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, time::Duration};

use crate::utils::validate;

pub const MAX_SESSION_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 14);

#[derive(Debug, Serialize, Deserialize)]
pub enum MaxMindEdition {
    #[serde(rename = "GeoLite2-City")]
    GeoLite2City,
    #[serde(rename = "GeoLite2-Country")]
    GeoLite2Country,
    #[serde(rename = "GeoLite2-ASN")]
    GeoLite2ASN,
    #[serde(rename = "GeoIP2-City")]
    GeoIP2City,
    #[serde(rename = "GeoIP2-Country")]
    GeoIP2Country,
}

impl TryFrom<&str> for MaxMindEdition {
    type Error = eyre::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GeoLite2-City" => Ok(Self::GeoLite2City),
            "GeoLite2-Country" => Ok(Self::GeoLite2Country),
            "GeoLite2-ASN" => Ok(Self::GeoLite2ASN),
            "GeoIP2-City" => Ok(Self::GeoIP2City),
            "GeoIP2-Country" => Ok(Self::GeoIP2Country),
            _ => bail!("Invalid MaxMind edition"),
        }
    }
}

fn default_db_path() -> String {
    "liwan.db".to_string()
}

fn default_port() -> u16 {
    3000
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_db_path")]
    pub db_path: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub geoip: Option<GeoIpConfig>,

    #[serde(default, rename = "user")]
    pub users: Vec<User>,

    #[serde(default, rename = "group")]
    pub groups: Vec<Group>,

    #[serde(default, rename = "entity")]
    pub entities: Vec<Entity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoIpConfig {
    pub maxmind_db_path: Option<String>,
    pub maxmind_account_id: Option<String>,
    pub maxmind_license_key: Option<String>,
    pub maxmind_edition: Option<MaxMindEdition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    pub display_name: String,
    pub entities: Vec<String>,

    #[serde(default)]
    pub public: bool,
    pub password: Option<String>, // enable public access with password protection
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub groups: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "readonly")]
    ReadOnly,
}

static DEFAULT_CONFIG: &str = include_str!("../config.example.toml");

impl Config {
    pub fn from_file(file: impl AsRef<Path>) -> Result<Self> {
        if std::fs::metadata(file.as_ref()).is_err() {
            std::fs::write("liwan.config.toml", DEFAULT_CONFIG).wrap_err("Failed to write default config file")?;
            println!("Config file not found, default config written to {}", file.as_ref().display());
        }

        let config = std::fs::read_to_string(file).wrap_err("Failed to read config file")?;
        let cfg: Self = toml::from_str(&config).wrap_err("Failed to parse config file")?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn refresh(&mut self) -> Result<()> {
        let config = std::fs::read_to_string("liwan.config.toml").wrap_err("Failed to read config file")?;
        let cfg: Self = toml::from_str(&config).wrap_err("Failed to parse config file")?;
        cfg.validate()?;
        *self = cfg;
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        let mut group_ids = std::collections::HashSet::new();
        let mut user_ids = std::collections::HashSet::new();
        let mut entity_ids = std::collections::HashSet::new();

        for entity in &self.entities {
            if !validate::is_valid_id(&entity.id) {
                bail!("Invalid entity id: {}", entity.id);
            }

            if !entity_ids.insert(&entity.id) {
                bail!("Duplicate entity id: {}", entity.id);
            }
        }

        for group in &self.groups {
            if !validate::is_valid_id(&group.id) {
                bail!("Invalid group id: {}", group.id);
            }

            if !group_ids.insert(&group.id) {
                bail!("Duplicate group id: {}", group.id);
            }

            for entity in &group.entities {
                if !entity_ids.contains(entity) {
                    bail!("Group {} has invalid entity: {}", group.id, entity);
                }
            }
        }

        for user in &self.users {
            if !validate::is_valid_id(&user.username) {
                bail!("Invalid username: {}", user.username);
            }

            if !user_ids.insert(&user.username) {
                bail!("Duplicate username: {}", user.username);
            }

            for group in &user.groups {
                if !group_ids.contains(group) {
                    bail!("User {} has invalid group", user.username);
                }
            }
        }

        Ok(())
    }

    pub fn resolve_entity(&self, id: &str) -> Option<Entity> {
        self.entities.iter().find(|&entity| entity.id == id).cloned()
    }

    pub fn resolve_entities(&self, ids: &[String]) -> BTreeMap<String, String> {
        self.entities
            .iter()
            .filter(|entity| ids.contains(&entity.id))
            .map(|entity| (entity.id.clone(), entity.display_name.clone()))
            .collect()
    }

    pub fn resolve_user(&self, username: &str) -> Option<User> {
        self.users.iter().find(|&user| user.username == username).cloned()
    }

    pub fn resolve_public_groups(&self) -> Vec<Group> {
        self.groups.iter().filter(|group| group.public && group.password.is_none()).cloned().collect()
    }

    pub fn has_access_to_entity(&self, user: Option<&User>, entity: &Entity) -> bool {
        let groups = self.resolve_user_groups(user);
        groups.iter().any(|group| group.entities.contains(&entity.id))
    }

    pub fn has_access_to_group(&self, user: Option<&User>, group: &Group) -> bool {
        let groups = self.resolve_user_groups(user);
        groups.iter().any(|g| g.id == group.id)
    }

    pub fn resolve_user_group(&self, id: &str, user: Option<&User>) -> Option<Group> {
        self.resolve_user_groups(user).iter().find(|&group| group.id == id).cloned()
    }

    pub fn resolve_user_groups(&self, user: Option<&User>) -> Vec<Group> {
        let Some(user) = user else {
            return self.resolve_public_groups();
        };
        if user.role == crate::config::UserRole::Admin {
            return self.groups.to_vec();
        }
        self.groups.iter().filter(|group| user.groups.contains(&group.id)).cloned().collect()
    }
}
