use std::{borrow::Cow, collections::HashSet, sync::Arc};

use log::*;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    stream::DuplexStream,
};
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex,
};

pub mod client;
pub mod usernamemgr;

use crate::{dropvec::DropVec, Config};
use client::{Client, ClientFactory, ClientInfo, Message};
use lmetrics::metrics;
use thiserror::Error;
use usernamemgr::{NameLease, UserId, UsernameManager};

metrics! {
    pub counter joined_total("Total joined users",[]);
    pub counter left_total("Total left users", []);
    pub counter messages_total("Total count of messages sent", []);
}

#[derive(Debug, Error)]
pub enum NewClientError {
    #[error("Max concurrent user count reached")]
    MaxConcurrentUserCount,
    #[error("Setup packet fail: {0}")]
    SetupPacketError(#[from] rocket_ws::result::Error),
}

pub struct Chat {
    messages_sender: broadcast::Sender<Message>,
    join_sender: broadcast::Sender<ClientInfo>,
    left_sender: broadcast::Sender<ClientInfo>,

    clients: Arc<Mutex<HashSet<ClientInfo>>>,
    history: Arc<Mutex<DropVec<Message>>>,
    unmgr: UsernameManager,
    client_factory: ClientFactory,

    config: Config,
}
impl Chat {
    pub fn new(config: Config) -> Self {
        let (messages_sender, messages_receiver) = broadcast::channel(20);
        let (join_sender, _) = broadcast::channel(20);
        let (left_sender, left_receiver) = broadcast::channel(20);

        let clients = Arc::new(Mutex::new(HashSet::new()));
        let history = Arc::new(Mutex::new(DropVec::new(config.max_stored_messages)));

        Self::spawn_histrec(
            left_receiver,
            messages_receiver,
            clients.clone(),
            history.clone(),
        );

        Self {
            messages_sender,
            join_sender,
            left_sender,
            clients,
            history,
            unmgr: UsernameManager::new(config.max_reserved_names),
            client_factory: ClientFactory::new(),
            config,
        }
    }

    fn spawn_histrec(
        mut left_receiver: broadcast::Receiver<ClientInfo>,
        mut messages_receiver: broadcast::Receiver<Message>,
        clients: Arc<Mutex<HashSet<ClientInfo>>>,
        history: Arc<Mutex<DropVec<Message>>>,
    ) {
        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    left_client = left_receiver.recv() => {
                        match left_client{
                            Ok(left_client)=>{
                                left_total::inc();
                                trace!("User {} left", left_client.id());
                                clients.lock().await.remove(&left_client);
                            },
                            Err(RecvError::Closed)=>{
                                return;
                            },
                            Err(RecvError::Lagged(count))=>{
                                error!("main client_left receiver lagged behind {} times. Ghosts will appear", count);
                            }
                        }
                    },
                    mesg = messages_receiver.recv() => {
                        match mesg{
                            Ok(mesg) => {
                                history.lock().await.push(mesg);
                                messages_total::inc();
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
        });
    }

    pub async fn new_client(
        &mut self,
        mut ws: DuplexStream,
        key: UserId,
        leased_name: NameLease,
    ) -> Result<Client, NewClientError> {
        if self.config.max_users != 0
            && self.config.max_users <= self.clients.lock().await.len() as u16
        {
            ws.close(Some(CloseFrame {
                code: CloseCode::Again,
                reason: Cow::Borrowed("Chat zit vol."),
            }))
            .await?;
            return Err(NewClientError::MaxConcurrentUserCount);
        }
        let client = self
            .client_factory
            .new_client(ws, key, leased_name, &self)
            .await
            .map_err(|e| NewClientError::SetupPacketError(e))?;
        let _ = self.join_sender.send(client.client_info()); // throws error when no receivers
        self.clients.lock().await.insert(client.client_info());

        Ok(client)
    }

    pub fn unmgr_mut(&mut self) -> &mut UsernameManager {
        &mut self.unmgr
    }
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn history<'a>(&'a self) -> Vec<Message> {
        self.history.lock().await.iter().cloned().collect()
    }
    pub async fn clients<'a>(&'a self) -> Vec<ClientInfo> {
        self.clients.lock().await.iter().cloned().collect()
    }

    pub fn subscribe_events(
        &self,
    ) -> (
        broadcast::Receiver<Message>,
        broadcast::Sender<Message>,
        broadcast::Receiver<ClientInfo>,
    ) {
        (
            self.messages_sender.subscribe(),
            self.messages_sender.clone(),
            self.join_sender.subscribe(),
        )
    }
    fn left_sender(&self) -> broadcast::Sender<ClientInfo> {
        self.left_sender.clone()
    }
}
