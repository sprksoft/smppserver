use rocket::{fairing::AdHoc, get, launch, routes};

//pub mod config;
// pub mod chat;
// pub mod client;
// pub mod dropvec;
// pub mod http;
// pub mod usernamemgr;

use lmetrics::LMetrics;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub metrics_addr: String,
    pub listen_addr: String,
    pub max_stored_messages: usize,
    pub name_reserve_time: u64,
    pub max_users: u16,
}

#[get("/socket/v1")]
fn socket_v1(ws: ws::WebSocket) -> ws::Channel<'static> {
    use rocket::futures::{SinkExt, StreamExt};

    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = stream.next().await {
                let _ = stream.send(message?).await;
            }

            Ok(())
        })
    })
}

#[launch]
fn rocket() -> _ {
    let mut metrics = LMetrics::new(&[
        // &chat::joined_total::METRIC,
        // &chat::left_total::METRIC,
        // &chat::messages_total::METRIC,
    ]);
    metrics.on_before_handle(|| {});
    rocket::build()
        .mount("/", routes![socket_v1])
        .mount("/metrics", metrics)
        .attach(AdHoc::config::<config::Config>())
}
