use std::{borrow::Cow, collections::HashSet, fmt::Display, num::NonZeroU32, str::FromStr};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Event {
    pub entity_id: String,
    pub visitor_group_id: String,
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
    pub screen_width: Option<String>,
    pub orientation: Option<String>,
    pub properties: EventProperties,
    pub track_sessions: bool,
}

pub type EventProperties = BTreeMap<String, String>;

impl Event {
    /// Serialize properties as a DuckDB map literal, since `duckdb-rs` cannot bind MAP values.
    // TODO: duckdb-neo should support MAP values directly
    pub fn properties_literal(&self) -> Option<String> {
        if self.properties.is_empty() {
            return None;
        }
        let quote = |literal: &mut String, text: &str| {
            literal.push('\'');
            for c in text.chars() {
                if matches!(c, '\\' | '\'') {
                    literal.push('\\');
                }
                literal.push(c);
            }
            literal.push('\'');
        };
        let mut literal = String::from("{");
        for (i, (key, value)) in self.properties.iter().enumerate() {
            if i > 0 {
                literal.push_str(", ");
            }
            quote(&mut literal, key);
            literal.push('=');
            quote(&mut literal, value);
        }
        literal.push('}');
        Some(literal)
    }
}

/// Whether an entity has ever recorded custom events or custom properties.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CustomData {
    pub events: bool,
    pub properties: bool,
}

/// Identifies a stored event that should receive an exit timestamp.
#[derive(Debug, Clone)]
pub struct EventExit {
    pub entity_id: String,
    pub visitor_group_id: String,
    pub event: String,
    pub created_at: DateTime<Utc>,
    pub fqdn: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: String,
    pub display_name: String,
    pub visibility: ProjectVisibility,
    pub secret: Option<String>, // currently unused
}

/// Whether and where a project is visible.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectVisibility {
    Private,
    Public,
    Unlisted,
    Internal,
}

impl rusqlite::types::FromSql for ProjectVisibility {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "private" => Ok(Self::Private),
            "public" => Ok(Self::Public),
            "unlisted" => Ok(Self::Unlisted),
            "internal" => Ok(Self::Internal),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl rusqlite::types::ToSql for ProjectVisibility {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(match self {
            Self::Private => "private",
            Self::Public => "public",
            Self::Unlisted => "unlisted",
            Self::Internal => "internal",
        }
        .into())
    }
}

#[derive(Debug, Clone)]
pub struct Team {
    pub id: String,
    pub display_name: String,
    pub users: Vec<String>,
    pub access: Access,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: String,
    pub display_name: String,
    pub entities: AccessScope,
    pub projects: AccessScope,
    pub permissions: Vec<AccessPermission>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// A scope covering every resource or a selected set of IDs.
#[derive(Debug, Clone)]
pub enum AccessScope {
    All,
    Selected(Vec<String>),
}

impl Default for AccessScope {
    fn default() -> Self {
        Self::Selected(Vec::new())
    }
}

impl AccessScope {
    /// Build a scope from a stored "all" flag and a JSON array of selected IDs.
    pub fn from_db(all: bool, ids: &str) -> serde_json::Result<Self> {
        if all { Ok(Self::All) } else { serde_json::from_str(ids).map(Self::Selected) }
    }

    /// Check whether an ID falls within this scope.
    pub fn contains(&self, id: &str) -> bool {
        match self {
            Self::All => true,
            Self::Selected(ids) => ids.iter().any(|selected| selected == id),
        }
    }
}

impl Serialize for AccessScope {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => serializer.serialize_str("all"),
            Self::Selected(ids) => ids.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for AccessScope {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == "all" {
            Ok(Self::All)
        } else {
            serde_json::from_value::<Vec<String>>(value).map(Self::Selected).map_err(serde::de::Error::custom)
        }
    }
}

impl JsonSchema for AccessScope {
    fn schema_name() -> Cow<'static, str> {
        "AccessScope".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        serde_json::json!({"anyOf": [{"const": "all", "type": "string"}, {"type": "array", "items": {"type": "string"}}]})
            .try_into()
            .expect("valid access scope schema")
    }
}

/// An action that may be granted within a resource scope.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
pub enum AccessPermission {
    #[serde(rename = "events:batch")]
    EventsBatch,
    #[serde(rename = "project:read")]
    ProjectRead,
    #[serde(rename = "entity:read")]
    EntityRead,
}

/// Resolved scopes and permissions for a user session or API key.
#[derive(Debug, Clone, Default)]
pub struct Access {
    pub entities: AccessScope,
    pub projects: AccessScope,
    pub permissions: HashSet<AccessPermission>,
}

impl Access {
    /// Check whether a permission covers a project.
    pub fn can_access_project(&self, id: &str, permission: AccessPermission) -> bool {
        permission == AccessPermission::ProjectRead
            && self.permissions.contains(&permission)
            && self.projects.contains(id)
    }

    /// Check whether a permission covers an entity.
    pub fn can_access_entity(&self, id: &str, permission: AccessPermission) -> bool {
        matches!(permission, AccessPermission::EventsBatch | AccessPermission::EntityRead)
            && self.permissions.contains(&permission)
            && self.entities.contains(id)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyExpiration {
    Never,
    #[serde(rename = "7_days")]
    SevenDays,
    #[serde(rename = "30_days")]
    ThirtyDays,
    #[serde(rename = "60_days")]
    SixtyDays,
    #[serde(rename = "90_days")]
    NinetyDays,
}

impl ApiKeyExpiration {
    pub fn expires_at(self) -> Option<DateTime<Utc>> {
        let days = match self {
            Self::Never => return None,
            Self::SevenDays => 7,
            Self::ThirtyDays => 30,
            Self::SixtyDays => 60,
            Self::NinetyDays => 90,
        };
        Some(Utc::now() + chrono::Duration::days(days))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VisitorGroupMode {
    #[default]
    Accurate,
    RandomPerRequest,
    NetworkStandard,
    NetworkBalanced,
    NetworkAccurate,
}

impl VisitorGroupMode {
    pub fn cidr_prefixes(self) -> Option<(u8, u8)> {
        match self {
            Self::NetworkStandard => Some((24, 56)),
            Self::NetworkBalanced => Some((28, 64)),
            Self::NetworkAccurate => Some((32, 128)),
            Self::Accurate | Self::RandomPerRequest => None,
        }
    }
}

impl Display for VisitorGroupMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Accurate => "accurate",
            Self::RandomPerRequest => "random_per_request",
            Self::NetworkStandard => "network_standard",
            Self::NetworkBalanced => "network_balanced",
            Self::NetworkAccurate => "network_accurate",
        })
    }
}

impl FromStr for VisitorGroupMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "accurate" => Ok(Self::Accurate),
            "random_per_request" => Ok(Self::RandomPerRequest),
            "network_standard" => Ok(Self::NetworkStandard),
            "network_balanced" => Ok(Self::NetworkBalanced),
            "network_accurate" => Ok(Self::NetworkAccurate),
            _ => Err(format!("invalid visitor group mode: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum GeoDetail {
    None,
    Country,
    #[default]
    City,
}

impl Display for GeoDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Country => "country",
            Self::City => "city",
        })
    }
}

impl FromStr for GeoDetail {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "country" => Ok(Self::Country),
            "city" => Ok(Self::City),
            _ => Err(format!("invalid geo detail: {value}")),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    IsNull,
    Equal,
    Contains,
    StartsWith,
    EndsWith,
    IsTrue,
    IsFalse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "mode", content = "days")]
pub enum DataRetention {
    Inherit,
    All,
    Days(NonZeroU32),
}

impl Display for FilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::IsNull => "is_null",
            Self::Equal => "equal",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::IsTrue => "is_true",
            Self::IsFalse => "is_false",
        })
    }
}

impl FromStr for FilterType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "is_null" => Ok(Self::IsNull),
            "equal" => Ok(Self::Equal),
            "contains" => Ok(Self::Contains),
            "starts_with" => Ok(Self::StartsWith),
            "ends_with" => Ok(Self::EndsWith),
            "is_true" => Ok(Self::IsTrue),
            "is_false" => Ok(Self::IsFalse),
            _ => Err(format!("invalid filter type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSettings {
    pub visitor_group_mode: VisitorGroupMode,
    pub track_sessions: bool,
    pub track_utm_params: bool,
    pub track_geo: GeoDetail,
    pub data_retention: DataRetention,
    pub ingest_drop_rules: Vec<IngestDropRule>,
}

impl Default for CollectionSettings {
    fn default() -> Self {
        Self {
            visitor_group_mode: VisitorGroupMode::Accurate,
            track_sessions: true,
            track_utm_params: true,
            track_geo: GeoDetail::City,
            data_retention: DataRetention::All,
            ingest_drop_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntityCollectionSettings {
    pub entity_id: String,
    pub visitor_group_mode: Option<VisitorGroupMode>,
    pub track_sessions: Option<bool>,
    pub track_utm_params: Option<bool>,
    pub track_geo: Option<GeoDetail>,
    pub data_retention: DataRetention,
    #[serde(default)]
    pub allowed_hostnames: Vec<String>,
    pub ingest_drop_rules: Vec<IngestDropRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCollectionSettings {
    pub visitor_group_mode: VisitorGroupMode,
    pub track_sessions: bool,
    pub track_utm_params: bool,
    pub track_geo: GeoDetail,
    pub data_retention: DataRetention,
    pub allowed_hostnames: Vec<String>,
    pub ingest_drop_rules: Vec<IngestDropRule>,
}

impl From<CollectionSettings> for ResolvedCollectionSettings {
    fn from(settings: CollectionSettings) -> Self {
        Self {
            visitor_group_mode: settings.visitor_group_mode,
            track_sessions: settings.track_sessions,
            track_utm_params: settings.track_utm_params,
            track_geo: settings.track_geo,
            data_retention: settings.data_retention,
            allowed_hostnames: Vec::new(),
            ingest_drop_rules: settings.ingest_drop_rules,
        }
    }
}

impl ResolvedCollectionSettings {
    pub fn resolve(global: CollectionSettings, entity: Option<EntityCollectionSettings>) -> Self {
        let Some(entity) = entity else {
            return global.into();
        };

        let mut ingest_drop_rules = global.ingest_drop_rules;
        ingest_drop_rules.extend(entity.ingest_drop_rules);

        Self {
            visitor_group_mode: entity.visitor_group_mode.unwrap_or(global.visitor_group_mode),
            track_sessions: entity.track_sessions.unwrap_or(global.track_sessions),
            track_utm_params: entity.track_utm_params.unwrap_or(global.track_utm_params),
            track_geo: entity.track_geo.unwrap_or(global.track_geo),
            data_retention: match entity.data_retention {
                DataRetention::Inherit => global.data_retention,
                retention => retention,
            },
            allowed_hostnames: entity.allowed_hostnames,
            ingest_drop_rules,
        }
    }
}

pub fn normalize_allowed_hostname_pattern(pattern: &str) -> Result<Option<String>, String> {
    let pattern = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    if pattern.is_empty() {
        return Ok(None);
    }

    if pattern.contains('*') {
        let Some(suffix) = pattern.strip_prefix("*.") else {
            return Err(format!("invalid hostname pattern: {pattern}"));
        };
        if suffix.contains('*') || !is_valid_hostname(suffix) {
            return Err(format!("invalid hostname pattern: {pattern}"));
        }
        return Ok(Some(format!("*.{suffix}")));
    }

    if !is_valid_hostname(&pattern) {
        return Err(format!("invalid hostname pattern: {pattern}"));
    }

    Ok(Some(pattern))
}

pub fn hostname_allowed(hostname: &str, allowed_hostnames: &[String]) -> bool {
    let hostname = hostname.trim_end_matches('.').to_ascii_lowercase();
    allowed_hostnames.is_empty()
        || allowed_hostnames.iter().any(|pattern| {
            if let Some(suffix) = pattern.strip_prefix("*.") {
                return hostname.len() > suffix.len()
                    && hostname.ends_with(suffix)
                    && hostname.as_bytes().get(hostname.len() - suffix.len() - 1) == Some(&b'.');
            }
            hostname == *pattern
        })
}

fn is_valid_hostname(hostname: &str) -> bool {
    !hostname.is_empty()
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestDropRule {
    pub filters: Vec<IngestFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngestFilter {
    pub dimension: String,
    pub filter_type: FilterType,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DisplayOverride {
    #[default]
    Auto,
    Show,
    Hide,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDisplaySettings {
    pub project_id: String,
    pub metric_display_overrides: BTreeMap<String, DisplayOverride>,
    pub dimension_display_overrides: BTreeMap<String, DisplayOverride>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub role: UserRole,
    /// Team scopes and permissions resolved when loading a session.
    pub access: Access,
}

impl User {
    /// Check whether this user can read a project through their role or teams.
    pub fn can_read_project(&self, id: &str) -> bool {
        self.role == UserRole::Admin || self.access.can_access_project(id, AccessPermission::ProjectRead)
    }

    /// Check whether this user can read an entity through their role or teams.
    pub fn can_read_entity(&self, id: &str) -> bool {
        self.role == UserRole::Admin || self.access.can_access_entity(id, AccessPermission::EntityRead)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_scopes_use_all_or_selected_ids() {
        assert_eq!(serde_json::to_value(AccessScope::All).unwrap(), serde_json::json!("all"));
        assert_eq!(
            serde_json::to_value(AccessScope::Selected(vec!["site".into()])).unwrap(),
            serde_json::json!(["site"])
        );
        assert!(matches!(serde_json::from_value::<AccessScope>(serde_json::json!("all")).unwrap(), AccessScope::All));
        assert!(serde_json::from_value::<AccessScope>(serde_json::json!("other")).is_err());
    }

    #[test]
    fn access_requires_a_matching_permission_and_scope() {
        let access = Access {
            entities: AccessScope::All,
            projects: AccessScope::Selected(vec!["site".into()]),
            permissions: HashSet::from([AccessPermission::ProjectRead]),
        };
        assert!(access.can_access_project("site", AccessPermission::ProjectRead));
        assert!(!access.can_access_project("other", AccessPermission::ProjectRead));
        assert!(!access.can_access_project("site", AccessPermission::EventsBatch));
        assert!(!access.can_access_entity("site", AccessPermission::ProjectRead));
        assert!(!access.can_access_entity("site", AccessPermission::EventsBatch));
        let access = Access { permissions: HashSet::from([AccessPermission::EntityRead]), ..access };
        assert!(access.can_access_entity("site", AccessPermission::EntityRead));
    }

    #[test]
    fn entity_retention_overrides_global_retention() {
        let resolved = ResolvedCollectionSettings::resolve(
            CollectionSettings { data_retention: DataRetention::All, ..Default::default() },
            Some(EntityCollectionSettings {
                entity_id: "entity".to_string(),
                visitor_group_mode: None,
                track_sessions: None,
                track_utm_params: None,
                track_geo: None,
                data_retention: DataRetention::Days(NonZeroU32::new(30).unwrap()),
                allowed_hostnames: Vec::new(),
                ingest_drop_rules: Vec::new(),
            }),
        );

        assert_eq!(resolved.data_retention, DataRetention::Days(NonZeroU32::new(30).unwrap()));
    }

    #[test]
    fn allowed_hostname_patterns_match_exact_and_wildcard_hosts() {
        let allowed_hostnames = vec!["example.com".to_string(), "*.example.org".to_string()];

        assert!(hostname_allowed("example.com", &allowed_hostnames));
        assert!(hostname_allowed("www.example.org", &allowed_hostnames));
        assert!(hostname_allowed("a.b.example.org", &allowed_hostnames));
        assert!(!hostname_allowed("www.example.com", &allowed_hostnames));
        assert!(!hostname_allowed("example.org", &allowed_hostnames));
    }

    #[test]
    fn empty_allowed_hostname_patterns_allow_all_hosts() {
        assert!(hostname_allowed("example.com", &[]));
    }

    #[test]
    fn invalid_allowed_hostname_patterns_are_rejected() {
        assert_eq!(normalize_allowed_hostname_pattern(" Example.COM. ").unwrap(), Some("example.com".to_string()));
        assert_eq!(normalize_allowed_hostname_pattern("*.Example.COM").unwrap(), Some("*.example.com".to_string()));
        assert_eq!(normalize_allowed_hostname_pattern("  ").unwrap(), None);
        assert!(normalize_allowed_hostname_pattern("example.*").is_err());
        assert!(normalize_allowed_hostname_pattern("*example.com").is_err());
        assert!(normalize_allowed_hostname_pattern("foo.*.example.com").is_err());
        assert!(normalize_allowed_hostname_pattern("-example.com").is_err());
    }
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
            $event.visitor_group_id,
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
            $event.screen_width,
            $event.orientation,
            None::<chrono::DateTime<chrono::Utc>>,
            $event.properties_literal(),
        ]
    };
}

pub use event_params;
