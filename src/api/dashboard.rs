use super::webext::*;
use crate::app::App;
use crate::reports::{self, DateRange, Metric, ReportStats};
use crate::utils::validate;

use poem::web::{Data, Path};
use poem::{http::StatusCode, session::Session};
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use std::collections::BTreeMap;

#[derive(Object)]
pub struct StatsRequest {
    pub range: DateRange,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub struct GraphRequest {
    pub range: DateRange,
    pub data_points: u32,
    pub metric: Metric,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct Group {
    pub display_name: String,
    pub entities: BTreeMap<String, String>,
    pub public: bool,
}

#[derive(Object)]
struct GroupsResponse {
    groups: BTreeMap<String, Group>,
}

#[derive(Object)]
struct GraphResponse {
    pub data: Vec<u32>,
}

pub struct DashboardAPI;

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/groups", method = "get")]
    async fn groups_handler(&self, Data(app): Data<&App>, session: &Session) -> poem::Result<Json<GroupsResponse>> {
        let user = get_user(session, app)?;
        let groups = app.config().resolve_user_groups(user.as_ref());

        let mut resp = BTreeMap::new();
        for group in groups {
            resp.insert(
                group.id.clone(),
                Group {
                    display_name: group.display_name.clone(),
                    entities: app.config().resolve_entities(&group.entities),
                    public: group.public,
                },
            );
        }

        Ok(Json(GroupsResponse { groups: resp }))
    }

    #[oai(path = "/group/:group_id/graph", method = "post")]
    async fn group_graph_handler(
        &self,
        Json(req): Json<GraphRequest>,
        Path(group_id): Path<String>,
        Data(app): Data<&App>,
        session: &Session,
    ) -> APIResult<Json<GraphResponse>> {
        if req.data_points > validate::MAX_DATAPOINTS {
            http_bail!(StatusCode::BAD_REQUEST, "Too many data points")
        }

        let user = get_user(session, app)?;
        let conn = app.conn().http_internal("Failed to get connection")?;
        let group = app.config().resolve_user_group(&group_id, user.as_ref()).http_not_found("Group not found")?;

        let filters = &[];
        let report = reports::overall_report(
            &conn,
            &group.entities,
            "pageview",
            &req.range,
            req.data_points,
            filters,
            &req.metric,
        )
        .http_internal("Failed to generate report")?;
        Ok(Json(GraphResponse { data: report }))
    }

    #[oai(path = "/group/:group_id/stats", method = "post")]
    async fn group_stats_handler(
        &self,
        Json(req): Json<StatsRequest>,
        Path(group_id): Path<String>,
        Data(app): Data<&App>,
        session: &Session,
    ) -> APIResult<Json<ReportStats>> {
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

        Ok(Json(stats))
    }
}
