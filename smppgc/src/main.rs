use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use chat::Chat;
use lmetrics::LMetrics;
use rocket::get;
use rocket::response::Redirect;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::{fairing::AdHoc, launch};
use tokio::sync::Mutex;

pub mod chat;
mod db;
#[cfg(debug_assertions)]
mod debug;
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
    pub max_reserved_names: u16,
    pub max_users: u16,
    pub rate_limit: RateLimitConfig,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct OfflineConfig {
    pub offline: bool,
}

pub struct ListenAddress {
    pub listen_address: SocketAddr,
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
        .attach(db::stage())
        .attach(static_routing::stage())
        .attach(template::stage())
        .attach(AdHoc::config::<OfflineConfig>())
        .attach(AdHoc::on_ignite("chat", |r| async {
            let config = r
                .figment()
                .extract::<Config>()
                .expect("No chat config found");

            r.mount("/", routes![socket::socket_v1])
                .manage(Arc::new(Mutex::new(Chat::new(config))))
        }));
    #[cfg(debug_assertions)]
    let r = r.attach(debug::stage());

    let ip = r.figment().extract_inner::<IpAddr>("address").unwrap();
    let port = r.figment().extract_inner::<u16>("port").unwrap();
    r.manage(ListenAddress {
        listen_address: SocketAddr::new(ip, port),
    })
}
