use super::webext::*;
use crate::app::App;
use crate::reports::{self, DateRange, Dimension, Metric};

use poem::handler;
use poem::web::Data;
use poem::{http::StatusCode, session::Session, IntoResponse};
use std::collections::HashMap;

#[handler]
pub(super) async fn test_handler(Data(app): Data<&App>) -> poem::Result<impl IntoResponse> {
    let res = reports::dimension_report(
        &app.conn().unwrap(),
        &["blog"],
        "pageview",
        DateRange { start: chrono::Utc::now() - chrono::Duration::days(7), end: chrono::Utc::now() },
        Dimension::Path,
        &[],
        Metric::UniqueVisitors,
    )
    .unwrap();
    http_res!(res)
}

#[derive(serde::Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub display_name: String,
    pub entities: HashMap<String, String>,
}

#[handler]
pub(super) async fn groups_handler(
    Data(app): Data<&App>,
    session: &Session,
) -> poem::Result<impl IntoResponse> {
    let user = auth(session, app)?;
    let groups = app.resolve_user_groups(user).http_err("user not found", StatusCode::NOT_FOUND)?;
    http_res!(groups
        .iter()
        .map(|g| GroupResponse {
            id: g.id.clone(),
            display_name: g.display_name.clone(),
            entities: app.resolve_entities(&g.entities),
        })
        .collect::<Vec<_>>())
}
