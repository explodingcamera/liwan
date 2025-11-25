#![allow(unused)]

use cookie::Cookie;
use liwan::{
    app::{Liwan, models::Event},
    config::Config,
};

pub fn app() -> std::sync::Arc<Liwan> {
    Liwan::new_memory(Config::default()).unwrap()
}

pub fn events() -> (std::sync::mpsc::Sender<Event>, std::sync::mpsc::Receiver<Event>) {
    std::sync::mpsc::channel::<Event>()
}

pub use liwan::web::create_router as router;
use poem::{Endpoint, IntoEndpoint, test::TestResponse};

pub fn cookies(res: &TestResponse) -> Vec<cookie::Cookie<'static>> {
    res.0
        .headers()
        .get_all("Set-Cookie")
        .iter()
        .map(|v| Cookie::parse(v.to_str().unwrap().to_owned()).unwrap())
        .collect::<Vec<_>>()
}

pub fn cookie_header(cookies: &[Cookie]) -> String {
    cookies.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
}

pub async fn login<T: Endpoint>(
    client: &poem::test::TestClient<T>,
    username: &str,
    password: &str,
) -> Vec<cookie::Cookie<'static>> {
    let login = &serde_json::json!({ "username": username, "password": password });
    let res = client.post("/api/dashboard/auth/login").body_json(login).send().await;
    cookies(&res)
}
