use std::collections::HashSet;

use client::{Client, Message};
use log::*;
use stderrlog::{LogLevelNum, StdErrLog};

mod client;
mod config;
mod dropvec;
mod key;

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
    },
};

use crate::{
    client::{ClientFactory, ClientInfo, RecieverError},
    dropvec::DropVec,
    key::Key,
};

/* async fn handle_thread(client_reciever: Receiver<(ClientSender, ClientReciever)>) {
    let mut senders = Vec::<ClientSender>::new();
    let mut messages = Vec::new();
    loop {
        if let Some((client_sender, client_reciever)) = client_reciever.try_recv().ok() {
            let client = client_sender.client();
            debug!("new user joined {}", client_sender.id());
            for sender in senders.iter_mut() {
                sender.forward_client(&client).await;
                client_sender.forward_client(&sender.client()).await;
            }
            senders.push(client_sender);
            tokio::spawn(async {
                loop {
                    if let Some(mesg) = client_reciever.try_recv().await {
                        if mesg.is_valid() && !mesg.is_empty() {
                            let client = client_reciever.client();
                            trace!("got message from {}: {}", client.id(), mesg.content);
                            for sender in senders {
                                sender.forward(&mesg).await;
                            }
                            messages.push(mesg)
                        }
                    }
                }
            });
        }
    }
} */

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

async fn handle_connection(
    stream: TcpStream,
    client_factory: &mut ClientFactory,
    clients: &mut HashSet<ClientInfo>,
    messages_sender: Sender<Message>,
    join_sender: Sender<ClientInfo>,
    left_sender: Sender<ClientInfo>,
) {
    let mut key = None;
    match accept_hdr_async(stream, |request: &Request, response: Response| {
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
    })
    .await
    {
        Ok(ws) => {
            let key = if let Some(key) = key {
                key
            } else {
                trace!("generating new key");
                Key::new()
            };
            trace!("new connection with key: {}", key);
            let mut client = match client_factory.new_client(ws, key).await {
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to create client: {}", err);
                    return;
                }
            };
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

    let (messages_sender, _) = broadcast::channel(16);
    let (join_sender, _) = broadcast::channel(16);
    let (left_sender, mut left_receiver) = broadcast::channel(16);

    let mut clients = HashSet::new();

    let server = TcpListener::bind(&config.listen_addr).await.unwrap();
    let mut client_factory = ClientFactory::new();
    println!("Listening on {}", &config.listen_addr);

    loop {
        tokio::select! {
            Ok((stream, _)) = server.accept() => {
                handle_connection(stream, &mut client_factory, &mut clients, messages_sender.clone(), join_sender.clone(), left_sender.clone()).await;
            }
            left_client = left_receiver.recv() => {
                match left_client{
                    Ok(left_client)=>{
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
