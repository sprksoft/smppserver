use std::sync::Arc;

use chat::Chat;
use lmetrics::LMetrics;
use rocket::routes;
use rocket::serde::Deserialize;
use rocket::{fairing::AdHoc, launch};
use tokio::sync::Mutex;

pub mod chat;
pub mod dropvec;
pub mod socket;
pub mod static_routing;
mod template;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    pub max_stored_messages: usize,
    pub name_reserve_time: u64,
    pub max_users: u16,
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
