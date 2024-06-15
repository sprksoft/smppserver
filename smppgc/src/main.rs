use std::net::TcpListener;

use toml;

mod config;

use config::Config;

fn main() {
    let config = match Config::load() {
        Ok(val) => val,
        Err(err) => {
            panic!("FATAL: {}", err)
        }
    };
    let server = TcpListener::bind(&config.listen_addr).unwrap();
    println!("Listening on {}", &config.listen_addr);
    for stream in server.incoming() {
        spawn(move || match stream {
            Ok(stream) => {
                if let Err(err) = handle_client(stream) {
                    match err {
                        Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
                        e => error!("test: {}", e),
                    }
                }
            }
            Err(e) => error!("Error accepting stream: {}", e),
        });
    }
}
