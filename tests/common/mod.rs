#![allow(unused)]

use cookie::Cookie;
use liwan::{
    app::{models::Event, Liwan},
    config::Config,
};

pub fn app() -> liwan::app::Liwan {
    Liwan::new_memory(Config::default()).unwrap()
}

pub fn events() -> (crossbeam::channel::Sender<Event>, crossbeam::channel::Receiver<Event>) {
    crossbeam::channel::unbounded::<Event>()
}

pub use liwan::web::create_router as router;
use poem::test::TestResponse;

pub fn cookies(res: &TestResponse) -> Vec<cookie::Cookie<'static>> {
    res.0
        .headers()
        .get_all("Set-Cookie")
        .iter()
        .map(|v| Cookie::parse(v.to_str().unwrap().to_owned()).unwrap())
        .collect::<Vec<_>>()
}

pub fn cookie_header(cookies: &[Cookie]) -> String {
    cookies.iter().map(|cookie| cookie.to_string()).collect::<Vec<_>>().join("; ")
}
