use std::fmt::Display;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Event {
    pub entity_id: String,
    pub visitor_id: String,
    pub event: String,
    pub created_at: DateTime<Utc>,
    pub fqdn: Option<String>,
    pub path: Option<String>,
    pub referrer: Option<String>,
    pub platform: Option<String>,
    pub browser: Option<String>,
    pub mobile: Option<bool>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub utm_content: Option<String>,
    pub utm_term: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: String,
    pub display_name: String,
    pub public: bool,
    pub secret: Option<String>, // enable public access with password protection
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub role: UserRole,
    pub projects: Vec<String>,
}

#[derive(Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    #[default]
    User,
}

impl Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::User => write!(f, "user"),
        }
    }
}

impl TryFrom<String> for UserRole {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            _ => Err(format!("invalid role: {value}")),
        }
    }
}

#[macro_export]
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
            $event.utm_source,
            $event.utm_medium,
            $event.utm_campaign,
            $event.utm_content,
            $event.utm_term,
            None::<std::time::Duration>,
            None::<std::time::Duration>,
        ]
    };
}

pub use event_params;
