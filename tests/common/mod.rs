#![allow(unused)]

use axum_test::TestServer;
use cookie::Cookie;
use liwan::{
    app::{Liwan, models::Event},
    config::Config,
};
use serde_json::json;
use std::sync::Arc;

pub fn app() -> std::sync::Arc<Liwan> {
    Liwan::new_memory(Config::default()).unwrap()
}

pub fn events() -> (std::sync::mpsc::Sender<Event>, std::sync::mpsc::Receiver<Event>) {
    std::sync::mpsc::channel::<Event>()
}

pub struct TestClient {
    server: TestServer,
}

impl TestClient {
    pub fn new(app: Arc<Liwan>, events: std::sync::mpsc::Sender<Event>) -> Self {
        let router = liwan::web::router(app, events).unwrap();
        let server = TestServer::new(router).unwrap();
        Self { server }
    }

    pub async fn get(&self, path: &str) -> axum_test::TestResponse {
        self.server.get(path).await
    }

    pub async fn get_with_headers(&self, path: &str, headers: Vec<(String, String)>) -> axum_test::TestResponse {
        let mut request = self.server.get(path);
        for (key, value) in headers {
            if key.to_lowercase() == "cookie" {
                // Parse and add individual cookies
                for cookie_str in value.split(';').map(|s| s.trim()) {
                    if let Some((name, val)) = cookie_str.split_once('=') {
                        request = request.add_cookie(Cookie::new(name.trim(), val.trim()));
                    }
                }
            } else {
                request = request.add_header(
                    key.parse::<axum::http::HeaderName>().unwrap(),
                    value.parse::<axum::http::HeaderValue>().unwrap(),
                );
            }
        }
        request.await
    }

    pub async fn post(&self, path: &str, body: serde_json::Value) -> axum_test::TestResponse {
        self.server.post(path).json(&body).await
    }

    pub async fn post_with_headers(
        &self,
        path: &str,
        body: serde_json::Value,
        headers: Vec<(String, String)>,
    ) -> axum_test::TestResponse {
        let mut request = self.server.post(path).json(&body);
        for (key, value) in headers {
            if key.to_lowercase() == "cookie" {
                // Parse and add individual cookies
                for cookie_str in value.split(';').map(|s| s.trim()) {
                    if let Some((name, val)) = cookie_str.split_once('=') {
                        request = request.add_cookie(Cookie::new(name.trim(), val.trim()));
                    }
                }
            } else {
                request = request.add_header(
                    key.parse::<axum::http::HeaderName>().unwrap(),
                    value.parse::<axum::http::HeaderValue>().unwrap(),
                );
            }
        }
        request.await
    }
}

pub fn cookies(res: &axum_test::TestResponse) -> Vec<cookie::Cookie<'static>> {
    res.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok().and_then(|s| Cookie::parse(s.to_owned()).ok()))
        .collect::<Vec<_>>()
}

pub fn cookie_header(cookies: &[Cookie]) -> String {
    cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; ")
}

pub async fn login(client: &TestClient, username: &str, password: &str) -> Vec<cookie::Cookie<'static>> {
    let login = json!({ "username": username, "password": password });
    let res = client.post("/api/dashboard/auth/login", login).await;
    cookies(&res)
}
