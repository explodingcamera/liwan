pub mod admin;
pub mod auth;
pub mod dashboard;
pub mod event;

pub use admin::AdminAPI;
pub use auth::AuthApi;
pub use dashboard::DashboardAPI;
pub use event::EventApi;
use poem_openapi::OpenApiService;

pub fn event_service() -> OpenApiService<EventApi, ()> {
    OpenApiService::new(EventApi, "event api", "1.0").url_prefix("/api/")
}

pub fn dashboard_service() -> OpenApiService<(DashboardAPI, AdminAPI, AuthApi), ()> {
    OpenApiService::new((DashboardAPI, AdminAPI, AuthApi), "liwan dashboard api", "1.0").url_prefix("/api/dashboard")
}
