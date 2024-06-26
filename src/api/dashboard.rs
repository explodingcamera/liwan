use super::webext::*;
use crate::app::App;
use crate::reports::{self, DateRange, Metric};
use crate::utils::validate;

use poem::handler;
use poem::web::{Data, Json, Path};
use poem::{get, http::StatusCode, post, session::Session, IntoResponse};
use std::collections::BTreeMap;

pub fn router() -> poem::Route {
    poem::Route::new()
        .at("/groups", get(groups_handler))
        .at("/group/:group_id/stats", post(group_stats_handler))
        .at("/group/:group_id/graph", post(group_graph_handler))
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct StatsRequest {
    pub range: DateRange,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRequest {
    pub range: DateRange,
    pub data_points: u32,
    pub metric: Metric,
}

#[handler]
pub(super) async fn group_graph_handler(
    Json(req): Json<GraphRequest>,
    Path(group_id): Path<String>,
    Data(app): Data<&App>,
    session: &Session,
) -> poem::Result<impl IntoResponse> {
    if req.data_points > validate::MAX_DATAPOINTS {
        http_bail!(StatusCode::BAD_REQUEST, "Too many data points")
    }

    let user = get_user(session, app)?;
    let conn = app.conn().http_internal("Failed to get connection")?;
    let group = app.config().resolve_user_group(&group_id, user.as_ref()).http_not_found("Group not found")?;

    let filters = &[];
    let report =
        reports::overall_report(&conn, &group.entities, "pageview", &req.range, req.data_points, filters, &req.metric);

    http_res!(report.http_internal("Failed to generate report")?)
}

#[handler]
pub(super) async fn group_stats_handler(
    Json(req): Json<StatsRequest>,
    Path(group_id): Path<String>,
    Data(app): Data<&App>,
    session: &Session,
) -> poem::Result<impl IntoResponse> {
    let user = get_user(session, app)?;
    let conn = app.conn().http_internal("Failed to get connection")?;
    let group = app.config().resolve_user_group(&group_id, user.as_ref()).http_not_found("Group not found")?;

    let filters = &[];

    let stats = match reports::overall_stats(&conn, &group.entities, "pageview", &req.range, filters) {
        Ok(stats) => stats,
        Err(e) => {
            println!("Failed to generate stats: {}", e);
            http_bail!(StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate stats")
        }
    };

    http_res!(stats)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponse {
    pub display_name: String,
    pub entities: BTreeMap<String, String>,
    pub public: bool,
}

#[handler]
pub(super) async fn groups_handler(Data(app): Data<&App>, session: &Session) -> poem::Result<impl IntoResponse> {
    let user = get_user(session, app)?;
    let groups = app.config().resolve_user_groups(user.as_ref());

    let mut resp = BTreeMap::new();
    for group in groups {
        resp.insert(
            group.id.clone(),
            GroupResponse {
                display_name: group.display_name.clone(),
                entities: app.config().resolve_entities(&group.entities),
                public: group.public,
            },
        );
    }

    http_res!(resp)
}
