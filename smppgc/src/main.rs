use log::*;
use stderrlog::{LogLevelNum, StdErrLog};

mod chat;
mod client;
mod config;
mod dropvec;
mod http;
mod usernamemgr;

use config::Config;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    StdErrLog::new()
        .verbosity(LogLevelNum::Trace)
        .module(module_path!())
        .init()
        .unwrap();

    let config = match Config::load() {
        Ok(val) => val,
        Err(err) => {
            panic!("FATAL: Failed to load config\n{}", err)
        }
    };

    let server = TcpListener::bind(&config.listen_addr).await.unwrap();
    info!("Listening on {}", &config.listen_addr);

    let mut chat = chat::Chat::new(&config);
    loop {
        chat.tick(&server).await;
    }
}
