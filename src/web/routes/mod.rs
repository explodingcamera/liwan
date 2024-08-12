mod admin;
mod auth;
mod dashboard;
mod event;

pub(crate) use admin::AdminAPI;
pub(crate) use auth::AuthApi;
pub(crate) use dashboard::DashboardAPI;
pub(crate) use event::EventApi;
use poem_openapi::OpenApiService;

pub(crate) fn event_service() -> OpenApiService<EventApi, ()> {
    OpenApiService::new(EventApi, "event api", "1.0").url_prefix("/api/")
}

pub(crate) fn dashboard_service() -> OpenApiService<(DashboardAPI, AdminAPI, AuthApi), ()> {
    OpenApiService::new((DashboardAPI, AdminAPI, AuthApi), "liwan dashboard api", "1.0").url_prefix("/api/dashboard")
}
