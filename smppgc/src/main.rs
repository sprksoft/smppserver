use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{mpsc::channel, Mutex},
    time::Duration,
};

use log::*;
use stderrlog::{new, LogLevelNum, StdErrLog};
use threadpool::ThreadPool;

mod client;
mod config;
mod key;

use config::Config;
use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
    http::StatusCode,
    Message, WebSocket,
};

use crate::{
    client::{Client, ClientFactory},
    key::Key,
};

fn main() {
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

    let (sender, recv) = channel::<Client>();
    std::thread::spawn(move || {
        let mut clients = Vec::<Client>::new();
        let mut messages = Vec::new();
        let mut pending_messages = Vec::new();
        loop {
            for client in clients.iter_mut() {
                if !client.alive() {
                    continue;
                }
                if let Some(mesg) = client.try_recv() {
                    if !mesg.is_valid() || mesg.is_empty() {
                        continue;
                    }
                    trace!("got message from {}: {}", client.id(), mesg.content);
                    pending_messages.push(mesg);
                }
            }
            for client in clients.iter_mut() {
                if !client.alive() {
                    continue;
                }
                for mesg in pending_messages.iter() {
                    client.send(mesg)
                }
            }
            messages.extend(pending_messages.drain(..));
            //std::thread::sleep(Duration::from_secs(10));
            if let Some(client) = recv.try_recv().ok() {
                debug!("new user joined {}", client.id());
                clients.push(client);
            }
        }
    });
    let server = TcpListener::bind(&config.listen_addr).unwrap();
    let mut client_factory = ClientFactory::new();
    println!("Listening on {}", &config.listen_addr);
    for result in server.incoming() {
        match result {
            Ok(stream) => {
                let mut key = None;
                match accept_hdr(stream, |request: &Request, response: Response| {
                    if request.uri().path() != "/smpp/gc" {
                        let resp = Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Some("smppgc: Not Found.".into()))
                            .unwrap();
                        return Err(resp);
                    }
                    key = if let Some(key) = request.uri().query() {
                        if key.len() < 3 {
                            None
                        } else {
                            Some(Key::parse_str(&key[1..]).ok_or_else(|| {
                                Response::builder()
                                    .status(StatusCode::UNAUTHORIZED)
                                    .body(Some("smppgc: Unauthorized. Invalid key".into()))
                                    .unwrap()
                            })?)
                        }
                    } else {
                        None
                    };

                    Ok(response)
                }) {
                    Ok(mut ws) => {
                        let key = if let Some(key) = key {
                            key
                        } else {
                            trace!("generating new key");
                            Key::new()
                        };
                        trace!("new connection with key: {}", key);
                        ws.flush().unwrap();
                        sender.send(client_factory.new_client(ws, key)).unwrap();
                    }
                    Err(e) => {
                        error!("Failed to create websocket connection: {}", e);
                    }
                };
            }
            Err(e) => error!("Error accepting stream: {}", e),
        };
    }
}
