mod dimension;
mod graph;
mod shared;
mod stats;

pub use dimension::dimension_report;
pub use graph::{build_graph_buckets, overall_report};
pub use stats::{earliest_timestamp, online_users, overall_stats};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

pub use crate::app::models::FilterType;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Hash, PartialEq, Eq)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn prev(&self) -> Self {
        let duration = self.end - self.start;
        Self { start: self.start - duration, end: self.start }
    }

    pub fn ends_in_future(&self) -> bool {
        self.end > Utc::now()
    }

    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }
}

impl Display for DateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Views,
    UniqueVisitors,
    BounceRate,
    AvgTimeOnSite,
}

impl Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Views => "views",
            Self::UniqueVisitors => "unique_visitors",
            Self::BounceRate => "bounce_rate",
            Self::AvgTimeOnSite => "avg_time_on_site",
        })
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GraphInterval {
    Hour,
    Day,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Url,
    UrlEntry,
    UrlExit,
    Fqdn,
    Path,
    Referrer,
    Platform,
    Browser,
    Mobile,
    Country,
    City,
    UtmSource,
    UtmMedium,
    UtmCampaign,
    UtmContent,
    UtmTerm,
    ScreenWidth,
    Orientation,
}

impl Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Url => "url",
            Self::UrlEntry => "url_entry",
            Self::UrlExit => "url_exit",
            Self::Fqdn => "fqdn",
            Self::Path => "path",
            Self::Referrer => "referrer",
            Self::Platform => "platform",
            Self::Browser => "browser",
            Self::Mobile => "mobile",
            Self::Country => "country",
            Self::City => "city",
            Self::UtmSource => "utm_source",
            Self::UtmMedium => "utm_medium",
            Self::UtmCampaign => "utm_campaign",
            Self::UtmContent => "utm_content",
            Self::UtmTerm => "utm_term",
            Self::ScreenWidth => "screen_width",
            Self::Orientation => "orientation",
        })
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReportGraphPoint {
    pub bin_start: DateTime<Utc>,
    pub value: f64,
}

pub type ReportGraph = Vec<ReportGraphPoint>;
pub type ReportTable = BTreeMap<String, f64>;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReportStats {
    pub total_views: u64,
    pub unique_visitors: u64,
    pub bounce_rate: Option<f64>,
    pub avg_time_on_site: Option<f64>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct DimensionFilter {
    pub(super) dimension: Dimension,
    pub(super) filter_type: FilterType,
    pub(super) inversed: Option<bool>,
    pub(super) strict: Option<bool>,
    pub(super) value: Option<String>,
}
