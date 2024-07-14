#[derive(Debug, Clone)]
pub(crate) struct Event {
    pub(crate) entity_id: String,
    pub(crate) visitor_id: String,
    pub(crate) event: String,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    pub(crate) fqdn: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) referrer: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) browser: Option<String>,
    pub(crate) mobile: Option<bool>,
    pub(crate) country: Option<String>,
    pub(crate) city: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Project {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) public: bool,
    pub(crate) secret: Option<String>, // enable public access with password protection
}

#[derive(Debug, Clone)]
pub(crate) struct Entity {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct User {
    pub(crate) username: String,
    pub(crate) role: UserRole,
    pub(crate) projects: Vec<String>,
}

#[derive(Debug, Enum, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Default)]
#[oai(rename_all = "snake_case")]
pub(crate) enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    #[default]
    User,
}

impl TryFrom<String> for UserRole {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "admin" => Ok(UserRole::Admin),
            "user" => Ok(UserRole::User),
            _ => Err(format!("invalid role: {}", value)),
        }
    }
}

impl UserRole {
    #[allow(clippy::inherent_to_string)]
    pub(crate) fn to_string(self) -> String {
        match self {
            UserRole::Admin => "admin".to_string(),
            UserRole::User => "user".to_string(),
        }
    }
}

macro_rules! event_params {
    ($event:expr) => {
        duckdb::params![
            $event.entity_id,
            $event.visitor_id,
            $event.event,
            $event.created_at,
            $event.fqdn,
            $event.path,
            $event.referrer,
            $event.platform,
            $event.browser,
            $event.mobile,
            $event.country,
            $event.city,
        ]
    };
}

pub(crate) use event_params;
use poem_openapi::Enum;
use serde::{Deserialize, Serialize};
