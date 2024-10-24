use std::{collections::HashSet, sync::Arc};

use log::*;
use tokio::sync::{
    broadcast::{self, error::RecvError},
    Mutex,
};

mod message;
pub use message::*;

use crate::{
    names::ClaimedName,
    userinfo::UserInfo,
    utils::{dropvec::DropVec, IdCounter},
    ChatConfig,
};
use lmetrics::metrics;
use thiserror::Error;

metrics! {
    pub counter joined_total("Total joined users",[]);
    pub counter left_total("Total left users", []);
    pub counter messages_total("Total count of messages sent", []);
}

#[derive(Debug, Error)]
pub enum NewClientError {
    #[error("Max concurrent user count reached")]
    MaxConcurrentUserCount,
}

pub struct Chat {
    messages_sender: broadcast::Sender<Message>,
    join_sender: broadcast::Sender<UserInfo>,
    left_sender: broadcast::Sender<UserInfo>,

    clients: Arc<Mutex<HashSet<UserInfo>>>,
    history: Arc<Mutex<DropVec<Message>>>,
    client_ids: IdCounter,

    config: ChatConfig,
}
impl Chat {
    pub fn new(config: ChatConfig) -> Self {
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
            client_ids: IdCounter::new(),
            config: config.into(),
        }
    }

    fn spawn_histrec(
        mut left_receiver: broadcast::Receiver<UserInfo>,
        mut messages_receiver: broadcast::Receiver<Message>,
        clients: Arc<Mutex<HashSet<UserInfo>>>,
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

    pub async fn new_client(&self, leased_name: ClaimedName) -> Result<ChatClient, NewClientError> {
        if self.config.max_users != 0
            && self.config.max_users <= self.clients.lock().await.len() as u16
        {
            return Err(NewClientError::MaxConcurrentUserCount);
        }

        let id = self.client_ids.new_id();
        let user_info = UserInfo {
            username: leased_name.into(),
            id,
        };
        let client = ChatClient {
            user_info,
            left_sender: self.left_sender.clone(),
            message_sender: self.messages_sender.clone(),
            message_receiver: self.messages_sender.subscribe(),
            join_receiver: self.join_sender.subscribe(),
        };

        let _ = self.join_sender.send(client.user_info()); // throws error when no receivers
        self.clients.lock().await.insert(client.user_info());
        joined_total::inc();

        Ok(client)
    }

    pub async fn history<'a>(&'a self) -> Vec<Message> {
        self.history.lock().await.iter().cloned().collect()
    }
    pub async fn clients<'a>(&'a self) -> Vec<UserInfo> {
        self.clients.lock().await.iter().cloned().collect()
    }
}

pub struct ChatClient {
    user_info: UserInfo,
    left_sender: rocket::tokio::sync::broadcast::Sender<UserInfo>,
    message_sender: broadcast::Sender<Message>,
    pub message_receiver: broadcast::Receiver<Message>,
    pub join_receiver: broadcast::Receiver<UserInfo>,
}
impl ChatClient {
    #[inline]
    pub fn user_info(&self) -> UserInfo {
        self.user_info.clone()
    }

    #[inline]
    pub fn send(&self, mesg: Message) {
        let _ = self.message_sender.send(mesg);
    }
}
impl Drop for ChatClient {
    fn drop(&mut self) {
        match self.left_sender.send(self.user_info()) {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "Failed to send leave event (This will cause ghosts to appear): {}",
                    err
                )
            }
        };
    }
}
