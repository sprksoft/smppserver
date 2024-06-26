use std::{hash::Hash, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;

use log::*;
use thiserror::Error;
use tokio_tungstenite::{tungstenite, WebSocketStream};

use crate::usernamemgr::Key;

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: u16,
    pub content: Arc<str>,
}
impl Message {
    pub const USERID_SPECIAL: u16 = 0;
    pub const SUBID_SETUP: u8 = 0;
    pub const SUBID_USERJOIN: u8 = 1;
    pub fn is_valid(&self) -> bool {
        if self.content.as_bytes().len() > 30 {
            return false;
        }
        if self.is_empty() {
            return false;
        }
        true
    }
    pub fn is_empty(&self) -> bool {
        for char in self.content.chars() {
            if !char.is_whitespace() {
                return false;
            }
        }
        true
    }
    pub fn new_setup(key: Key, id: u16) -> tokio_tungstenite::tungstenite::Message {
        let key_str = key.to_string();
        let key_str_bytes = key_str.as_bytes();
        let mut data = Vec::with_capacity(key_str_bytes.len() + 3);
        data.extend_from_slice(&Self::USERID_SPECIAL.to_be_bytes());
        data.push(Self::SUBID_SETUP);
        data.extend_from_slice(&id.to_be_bytes());
        data.extend_from_slice(key_str_bytes);
        tokio_tungstenite::tungstenite::Message::Binary(data)
    }
    pub fn new_client_joined(client: &ClientInfo) -> tokio_tungstenite::tungstenite::Message {
        let username_bytes = client.username.as_bytes();
        let mut data = Vec::with_capacity(username_bytes.len() + 5);
        data.extend_from_slice(&Self::USERID_SPECIAL.to_be_bytes());
        data.push(Self::SUBID_USERJOIN);
        data.extend_from_slice(&client.id.to_be_bytes());
        data.extend_from_slice(&username_bytes);
        tokio_tungstenite::tungstenite::Message::Binary(data)
    }
    pub fn new_message(mesg: &Message) -> tokio_tungstenite::tungstenite::Message {
        let content_bytes = mesg.content.as_bytes();
        let mut data = Vec::with_capacity(content_bytes.len() + 2);
        data.extend_from_slice(&mesg.sender.to_be_bytes());
        data.extend_from_slice(content_bytes);
        tungstenite::Message::Binary(data)
    }
}

pub struct ClientFactory {
    id_counter: u16,
    //id_slots: Box<[u8; u16::MAX as usize / 8]>,
}
impl ClientFactory {
    pub fn new() -> Self {
        Self {
            id_counter: 0,
            //id_slots: Box::new([0; u16::MAX as usize / 8]),
        }
    }
    pub fn reserve_id(&mut self) -> u16 {
        self.id_counter = self.id_counter.overflowing_add(1).0;
        if self.id_counter == 0 {
            self.id_counter += 1;
        }
        self.id_counter
    }
    pub async fn new_client(
        &mut self,
        mut ws: WebSocketStream<TcpStream>,
        key: Key,
        username: String,
    ) -> Result<Client> {
        let id = self.reserve_id();
        ws.send(Message::new_setup(key, id)).await?;
        let info = ClientInfo {
            username: username.into(),
            id,
        };
        Ok(Client { ws, info })
    }
}

#[derive(Debug, Error)]
pub enum RecieverError {
    #[error("Invalid packet or protocol error")]
    Invalid,
    #[error("Client Disconnected")]
    Disconected,
}
impl From<tungstenite::Error> for RecieverError {
    fn from(err: tungstenite::Error) -> Self {
        match err {
            tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
                RecieverError::Disconected
            }
            _ => RecieverError::Invalid,
        }
    }
}
type Result<T> = std::result::Result<T, RecieverError>;
pub struct Client {
    ws: WebSocketStream<TcpStream>,
    info: ClientInfo,
}
impl Client {
    pub async fn forward_client(&mut self, client: &ClientInfo) -> Result<()> {
        self.ws.send(Message::new_client_joined(client)).await?;
        Ok(())
    }
    pub async fn forward_all_clients(
        &mut self,
        clients: impl Iterator<Item = &ClientInfo>,
    ) -> Result<()> {
        for client in clients {
            self.ws.feed(Message::new_client_joined(client)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn forward(&mut self, mesg: &Message) -> Result<()> {
        self.ws.send(Message::new_message(mesg)).await?;
        Ok(())
    }
    pub async fn forward_all(&mut self, messages: impl Iterator<Item = &Message>) -> Result<()> {
        for message in messages {
            self.ws.feed(Message::new_message(message)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn try_recv(&mut self) -> Result<Message> {
        let message = self.ws.next().await.ok_or(RecieverError::Invalid)??;
        if !message.is_text() {
            return Err(RecieverError::Invalid);
        }
        let content = String::from_utf8_lossy(&message.into_data()).to_string();
        Ok(Message {
            sender: self.info.id(),
            content: content.into(),
        })
    }

    pub fn client_info(&self) -> ClientInfo {
        self.info.clone()
    }
}

#[derive(Clone, Debug, Hash)]
pub struct ClientInfo {
    username: Arc<str>,
    id: u16,
}
impl Eq for ClientInfo {}
impl PartialEq for ClientInfo {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
    fn ne(&self, other: &Self) -> bool {
        other.id != self.id
    }
}
impl ClientInfo {
    pub fn id(&self) -> u16 {
        self.id
    }
}
