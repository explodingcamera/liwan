#![allow(unused)]

use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::ConnectInfo,
    http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, header},
};
use axum_extra::extract::cookie::Cookie;
use liwan::{
    app::{
        Liwan,
        models::{Event, EventExit},
    },
    config::Config,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;

pub struct EventReceivers {
    pub events: tokio::sync::mpsc::Receiver<Event>,
    pub exits: tokio::sync::mpsc::Receiver<EventExit>,
}

pub fn app() -> std::sync::Arc<Liwan> {
    Liwan::new_memory(Config::default()).unwrap()
}

pub fn events() -> (liwan::web::EventQueues, EventReceivers) {
    let (events, event_rx) = tokio::sync::mpsc::channel::<Event>(10240);
    let (exits, exit_rx) = tokio::sync::mpsc::channel::<EventExit>(10240);
    (liwan::web::EventQueues { events, exits }, EventReceivers { events: event_rx, exits: exit_rx })
}

pub struct TestClient {
    router: Router,
}

impl TestClient {
    pub fn new(app: Arc<Liwan>, queues: liwan::web::EventQueues) -> Self {
        let (router, _) = liwan::web::router(app, queues).unwrap();
        Self { router }
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        self.get_with_headers(path, vec![]).await
    }

    pub async fn get_with_headers(&self, path: &str, headers: Vec<(String, String)>) -> TestResponse {
        self.request(Method::GET, path, None, headers).await
    }

    pub async fn post(&self, path: &str, body: Value) -> TestResponse {
        self.post_with_headers(path, body, vec![]).await
    }

    pub async fn post_with_headers(&self, path: &str, body: Value, headers: Vec<(String, String)>) -> TestResponse {
        self.request(Method::POST, path, Some(body), headers).await
    }

    pub async fn put_with_headers(&self, path: &str, body: Value, headers: Vec<(String, String)>) -> TestResponse {
        self.request(Method::PUT, path, Some(body), headers).await
    }

    pub async fn delete_with_headers(&self, path: &str, headers: Vec<(String, String)>) -> TestResponse {
        self.request(Method::DELETE, path, None, headers).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        headers: Vec<(String, String)>,
    ) -> TestResponse {
        let mut request =
            Request::builder().method(method).uri(path).extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
        for (key, value) in headers {
            request = request.header(key, value);
        }
        let body = match body {
            Some(body) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(body.to_string())
            }
            None => Body::empty(),
        };
        let response = self.router.clone().oneshot(request.body(body).unwrap()).await.unwrap();
        let (parts, body) = response.into_parts();
        TestResponse(Response::from_parts(parts, to_bytes(body, usize::MAX).await.unwrap()))
    }
}

pub struct TestResponse(Response<Bytes>);

impl TestResponse {
    pub fn headers(&self) -> &HeaderMap {
        self.0.headers()
    }

    #[track_caller]
    pub fn header(&self, name: &str) -> &HeaderValue {
        &self.headers()[name]
    }

    #[track_caller]
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(self.0.body()).unwrap_or_else(|err| {
            panic!("Invalid JSON response: {err}, body: {}", String::from_utf8_lossy(self.0.body()))
        })
    }

    #[track_caller]
    pub fn assert_json(&self, expected: &Value) {
        assert_eq!(&self.json::<Value>(), expected);
    }

    #[track_caller]
    pub fn assert_header(&self, name: &str, value: &str) {
        assert_eq!(self.header(name), value);
    }

    #[track_caller]
    pub fn assert_status(&self, expected: StatusCode) {
        assert_eq!(self.0.status(), expected, "{}", String::from_utf8_lossy(self.0.body()));
    }

    #[track_caller]
    pub fn assert_status_success(&self) {
        assert!(self.0.status().is_success(), "{}: {}", self.0.status(), String::from_utf8_lossy(self.0.body()));
    }
}

pub fn cookies(res: &TestResponse) -> Vec<Cookie<'static>> {
    res.headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok().and_then(|s| Cookie::parse(s.to_owned()).ok()))
        .collect::<Vec<_>>()
}

pub fn cookie_header(cookies: &[Cookie]) -> String {
    cookies.iter().map(|c| format!("{}={}", c.name(), c.value())).collect::<Vec<_>>().join("; ")
}

pub async fn login(client: &TestClient, username: &str, password: &str) -> Vec<Cookie<'static>> {
    let login = json!({ "username": username, "password": password });
    let res = client.post("/api/dashboard/auth/login", login).await;
    res.assert_status_success();
    cookies(&res)
}
