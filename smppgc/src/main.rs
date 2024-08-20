use std::sync::Arc;

use chat::Chat;
use lmetrics::LMetrics;
use rocket::get;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::{fairing::AdHoc, launch};
use tokio::sync::Mutex;

pub mod chat;
pub mod dropvec;
pub mod socket;
pub mod static_routing;
mod template;

#[derive(Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct RateLimitConfig {
    pub min_message_time_hard: isize,
    pub min_message_time_soft: isize,
    pub kick_burst: isize,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    pub max_stored_messages: usize,
    pub name_reserve_time: u64,
    pub max_users: u16,
    pub rate_limit: RateLimitConfig,
}

#[get("/version")]
fn server_version() -> &'static str {
    if cfg!(debug_assertions) {
        concat!(env!("CARGO_PKG_NAME"), "-debug-", env!("CARGO_PKG_VERSION"))
    } else {
        concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"))
    }
}

#[launch]
fn rocket() -> _ {
    let mut metrics = LMetrics::new(&[
        &static_routing::static_req_total::METRIC,
        &chat::joined_total::METRIC,
        &chat::left_total::METRIC,
        &chat::messages_total::METRIC,
    ]);
    metrics.on_before_handle(|| {});
    rocket::build()
        .mount("/", routes![server_version])
        .mount("/metrics", metrics)
        .attach(static_routing::stage())
        .attach(template::stage())
        .attach(AdHoc::on_ignite("chat", |r| async {
            let config = r
                .figment()
                .extract::<Config>()
                .expect("No chat config found");

            r.mount("/", routes![socket::socket_v1])
                .manage(Arc::new(Mutex::new(Chat::new(config))))
        }))
}
