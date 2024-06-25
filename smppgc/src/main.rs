use std::{borrow::Cow, collections::HashSet};

use client::{Client, Message};
use log::*;
use stderrlog::{LogLevelNum, StdErrLog};

mod client;
mod config;
mod dropvec;
mod usernamemgr;

use config::Config;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, error::RecvError, Receiver, Sender},
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        http::StatusCode,
        protocol::{frame::coding::CloseCode, CloseFrame},
    },
    WebSocketStream,
};
use usernamemgr::NameLeaseError;

use crate::{
    client::{ClientFactory, ClientInfo, RecieverError},
    usernamemgr::Key,
    usernamemgr::UsernameManager,
};

async fn handle_client(
    mut client: Client,
    mut join_reciever: Receiver<ClientInfo>,
    left_sender: Sender<ClientInfo>,
    messages_sender: Sender<Message>,
) {
    let mut messages_receiver = messages_sender.subscribe();
    loop {
        tokio::select! {
            mesg = client.try_recv() => {
                match mesg {
                    Ok(mesg) => {
                        trace!("got message from {}: {}", mesg.sender, mesg.content);
                        let _ = messages_sender.send(mesg);
                    },
                    Err(RecieverError::Disconected) => {
                        break;
                    }
                    Err(err) =>{
                        error!("{}", err);
                        break;
                    }
                }
            }
            mesg = messages_receiver.recv() => {
                match mesg{
                    Ok(mesg) => {
                        match client.forward(&mesg).await {
                            Ok(_) => {},
                            Err(RecieverError::Disconected) => {
                                break;
                            }
                            Err(err) => {
                                error!("{}", err);
                                break;
                            }
                        };
                    }
                    Err(RecvError::Lagged(count)) => {
                        error!("{} Messages lost", count);
                    },
                    Err(RecvError::Closed)=>{
                        break;
                    }
                }
            }
            joined_client = join_reciever.recv() => {
                match joined_client{
                    Ok(joined_client) => {
                        trace!("user join {}", joined_client.id());
                        match client.forward_client(&joined_client).await {
                            Ok(_) => {},
                            Err(RecieverError::Disconected) => {
                                break;
                            }
                            Err(err) => {
                                error!("{}", err);
                                break;
                            },
                        }
                    },
                    Err(RecvError::Lagged(count)) => {
                        error!("{} Join messages lost", count);
                    }, Err(RecvError::Closed)=>{
                        break;
                    }
                }
            }
        }
    }
    let _ = left_sender.send(client.client_info());
}

async fn handle_client_preconnect(
    query: Option<String>,
    client_factory: &mut ClientFactory,
    unmgr: &mut UsernameManager,
    mut ws: WebSocketStream<TcpStream>,
) -> Option<Client> {
    macro_rules! err {
        ($code:expr, $reason:literal) => {
            let _ = ws
                .close(Some(CloseFrame {
                    code: $code,
                    reason: Cow::Borrowed($reason),
                }))
                .await;
            return None;
        };
    }
    let Some(query) = query else {
        err!(CloseCode::Error, "username required");
    };
    let (username, key) = if let Some(username_key_split) = query.rfind('&') {
        let key = match Key::parse_str(&query[username_key_split + 1..]) {
            Some(key) => key,
            None => {
                err!(CloseCode::Error, "Invalid key");
            }
        };
        (query[..username_key_split].to_string(), key)
    } else {
        trace!("Generating new key");
        (query, Key::new())
    };
    match unmgr.lease_name(&username, key.clone()) {
        Ok(_) => {}
        Err(NameLeaseError::Invalid) => {
            let _ = ws.close(Some(CloseFrame{
                code: CloseCode::Error,
                reason: Cow::Borrowed("Invalid username. Username must be between 3-10 characters and can only contain ascii characters")
            })).await;
            return None;
        }
        Err(NameLeaseError::Taken) => {
            let _ = ws
                .close(Some(CloseFrame {
                    code: CloseCode::Error,
                    reason: Cow::Borrowed("Username taken"),
                }))
                .await;
            return None;
        }
    };
    trace!("new connection with key: {}", key);
    Some(match client_factory.new_client(ws, key, username).await {
        Ok(val) => val,
        Err(err) => {
            error!("Failed to create client: {}", err);
            return None;
        }
    })
}

async fn handle_connection(
    stream: TcpStream,
    client_factory: &mut ClientFactory,
    clients: &mut HashSet<ClientInfo>,
    unmgr: &mut UsernameManager,
    config: &Config,
    messages_sender: Sender<Message>,
    join_sender: Sender<ClientInfo>,
    left_sender: Sender<ClientInfo>,
) {
    let mut query = None;
    match accept_hdr_async(stream, |request: &Request, response: Response| {
        if request.uri().path() != "/smpp/gc" {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Some("smppgc: Not Found.".into()))
                .unwrap();
            return Err(resp);
        }
        query = request.uri().query().map(|v| v.to_string());

        Ok(response)
    })
    .await
    {
        Ok(mut ws) => {
            if config.max_users != 0 && config.max_users <= clients.len() as u16 {
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Again,
                        reason: Cow::Borrowed("Server overloaded. Don't try again please."),
                    }))
                    .await;
                return;
            }
            let mut client = match handle_client_preconnect(query, client_factory, unmgr, ws).await
            {
                Some(client) => client,
                None => {
                    return;
                }
            };

            trace!("User joined {}", client.client_info().id());
            for other_client in clients.iter() {
                match client.forward_client(other_client).await {
                    Ok(_) => {}
                    Err(RecieverError::Invalid) => {
                        error!("Socket error when forwarding already present clients.");
                        return;
                    }
                    Err(RecieverError::Disconected) => {
                        return;
                    }
                };
            }

            clients.insert(client.client_info());
            //Send can only fail if no receivers
            let _ = join_sender.send(client.client_info());
            tokio::spawn(handle_client(
                client,
                join_sender.subscribe(),
                left_sender,
                messages_sender,
            ));
        }
        Err(err) => {
            error!("Failed to accept WebSocket connection: {}", err)
        }
    };
}

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

    let (messages_sender, _) = broadcast::channel(20);
    let (join_sender, _) = broadcast::channel(20);
    let (left_sender, mut left_receiver) = broadcast::channel(20);

    let mut clients = HashSet::new();
    let mut unmgr = UsernameManager::new(config.name_reserve_time);

    let server = TcpListener::bind(&config.listen_addr).await.unwrap();
    let mut client_factory = ClientFactory::new();
    println!("Listening on {}", &config.listen_addr);

    loop {
        tokio::select! {
            Ok((stream, _)) = server.accept() => {
                    handle_connection(stream, &mut client_factory, &mut clients, &mut unmgr, &config, messages_sender.clone(), join_sender.clone(), left_sender.clone()).await;
            }
            left_client = left_receiver.recv() => {
                match left_client{
                    Ok(left_client)=>{
                        trace!("User {} left", left_client.id());
                        clients.remove(&left_client);
                    },
                    Err(RecvError::Closed)=>{
                        return;
                    },
                    Err(RecvError::Lagged(count))=>{
                        panic!("main client_left receiver lagged behind {} times. Panicking because ghost clients will be left behind", count);
                    }
                }
            }

        }
    }
}
