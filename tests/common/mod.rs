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
