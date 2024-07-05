use std::{borrow::Cow, collections::HashSet};

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{self, error::TryRecvError};
use tokio_tungstenite::{
    tungstenite::protocol::{frame::coding::CloseCode, CloseFrame},
    WebSocketStream,
};

use crate::{
    client::{Client, ClientFactory, ClientInfo, Message, RecieverError},
    config::Config,
    dropvec::DropVec,
    http,
    usernamemgr::UsernameManager,
    usernamemgr::{Key, NameLeaseError},
};

use log::*;

pub struct Chat {
    messages_sender: broadcast::Sender<Message>,
    messages_receiver: broadcast::Receiver<Message>,
    join_sender: broadcast::Sender<ClientInfo>,
    left_receiver: broadcast::Receiver<ClientInfo>,
    left_sender: broadcast::Sender<ClientInfo>,

    clients: HashSet<ClientInfo>,
    history: DropVec<Message>,
    unmgr: UsernameManager,
    client_factory: ClientFactory,
    max_users: u16,
}
impl Chat {
    pub fn new(config: &Config) -> Self {
        let (messages_sender, messages_receiver) = broadcast::channel(20);
        let (join_sender, _) = broadcast::channel(20);
        let (left_sender, left_receiver) = broadcast::channel(20);

        Self {
            messages_sender,
            messages_receiver,
            join_sender,
            left_sender,
            left_receiver,
            clients: HashSet::new(),
            history: DropVec::new(config.max_stored_messages),
            unmgr: UsernameManager::new(config.name_reserve_time),
            client_factory: ClientFactory::new(),
            max_users: config.max_users,
        }
    }

    pub async fn tick(&mut self, server: &TcpListener) {
        tokio::select! {
            Ok((stream, _)) = server.accept() => {
                http::handle(stream, self).await;
            }
            left_client = self.left_receiver.recv() => {
                match left_client{
                    Ok(left_client)=>{
                        trace!("User {} left", left_client.id());
                        self.clients.remove(&left_client);
                    },
                    Err(RecvError::Closed)=>{
                        return;
                    },
                    Err(RecvError::Lagged(count))=>{
                        panic!("main client_left receiver lagged behind {} times. Panicking because ghost clients will be left behind", count);
                    }
                }
            },
            mesg = self.messages_receiver.recv() => {
                match mesg{
                    Ok(mesg) => {
                        self.history.push(mesg);
                    },
                    Err(RecvError::Closed) => {
                        return;
                    },
                    Err(RecvError::Lagged(count))=>{
                        error!("Lost {} messages", count);
                    }
                }
            }

        }
    }

    pub async fn handle_ws(&mut self, mut ws: WebSocketStream<TcpStream>, query: Option<String>) {
        if self.max_users != 0 && self.max_users <= self.clients.len() as u16 {
            let _ = ws
                .close(Some(CloseFrame {
                    code: CloseCode::Again,
                    reason: Cow::Borrowed(
                        "Max concurrent user count exceeded. Don't try again please.",
                    ),
                }))
                .await;
            return;
        }

        loop {
            match self.messages_receiver.try_recv() {
                Ok(mesg) => self.history.push(mesg),
                Err(TryRecvError::Closed) => {
                    break;
                }
                Err(TryRecvError::Lagged(count)) => {
                    error!("{} messages lost for history tracking.", count)
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
            }
        }
        let messages_receiver = self.messages_sender.subscribe();

        let client = match self.handle_client_preconnect(query, ws).await {
            Some(client) => client,
            None => {
                return;
            }
        };

        trace!("User joined {}", client.client_info().id());

        self.clients.insert(client.client_info());
        //Send can only fail if no receivers
        let _ = self.join_sender.send(client.client_info());
        tokio::spawn(handle_client(
            client,
            self.join_sender.subscribe(),
            self.left_sender.clone(),
            self.messages_sender.clone(),
            messages_receiver,
        ));
    }
    async fn handle_client_preconnect(
        &mut self,
        query: Option<String>,
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
        match self.unmgr.lease_name(&username, key.clone()) {
            Ok(_) => {}
            Err(NameLeaseError::Invalid) => {
                let _ = ws.close(Some(CloseFrame{
                code: CloseCode::Error,
                reason: Cow::Borrowed("Invalid username. Username must be between 2-15 characters and can only contain letters and numbers")
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
        Some(
            match self
                .client_factory
                .new_client(ws, key, username, self.clients.iter(), self.history.iter())
                .await
            {
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to create client: {}", err);
                    return None;
                }
            },
        )
    }
}

async fn handle_client(
    mut client: Client,
    mut join_reciever: broadcast::Receiver<ClientInfo>,
    left_sender: broadcast::Sender<ClientInfo>,
    messages_sender: broadcast::Sender<Message>,
    mut messages_receiver: broadcast::Receiver<Message>,
) {
    loop {
        tokio::select! {
            mesg = client.try_recv() => {
                match mesg {
                    Ok(mesg) => {
                        if mesg.is_valid(){
                            trace!("got message from {}: {}", mesg.sender, mesg.content);
                            let _ = messages_sender.send(mesg);
                        }
                    },
                    Err(RecieverError::Disconected) => {
                        break;
                    }
                    Err(err) =>{
                        warn!("Ungraceful disconnect: {}", err);
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
                                warn!("Ungraceful disconnect: {}", err);
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
                                warn!("Ungraceful disconnect: {}", err);
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
