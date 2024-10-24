use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chat::Chat;
use lmetrics::LMetrics;
use rocket::get;
use rocket::response::Redirect;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::{fairing::AdHoc, launch};
use tokio::sync::Mutex;
use utils::static_routing;

pub mod chat;
#[cfg(debug_assertions)]
mod debug;
mod mesg_filter;
pub mod names;
pub mod profanity;
pub mod ratelimit;
pub mod socket;
mod template;
mod userinfo;
mod utils;
mod wsprotocol;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ChatConfig {
    pub max_stored_messages: usize,
    pub max_users: u16,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct MaxLengthConfig {
    pub max_message_len: usize,
    pub max_username_len: usize,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct OfflineConfig {
    pub offline: bool,
}

#[get("/version")]
fn server_version() -> &'static str {
    if cfg!(debug_assertions) {
        concat!(env!("CARGO_PKG_NAME"), "-debug-", env!("CARGO_PKG_VERSION"))
    } else {
        concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"))
    }
}

#[get("/")]
fn index() -> Redirect {
    if cfg!(debug_assertions) {
        Redirect::permanent("/v1")
    } else {
        Redirect::permanent("/smpp/gc/v1")
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
    let r = rocket::build()
        .mount("/", routes![index, server_version])
        .mount("/metrics", metrics)
        .attach(static_routing::stage())
        .attach(template::stage())
        .attach(names::stage())
        .attach(AdHoc::config::<ratelimit::RateLimitConfig>())
        .attach(AdHoc::config::<OfflineConfig>())
        .attach(AdHoc::config::<MaxLengthConfig>())
        .attach(AdHoc::on_ignite("chat", |r| async {
            let config = r
                .figment()
                .extract::<ChatConfig>()
                .expect("No chat config found");

            r.mount("/", routes![socket::socket_v1])
                .manage(Chat::new(config))
        }));
    #[cfg(debug_assertions)]
    r.attach(debug::stage())
}
