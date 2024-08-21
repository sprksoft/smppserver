use std::{
    borrow::Cow,
    hash::Hash,
    sync::{atomic::AtomicU16, Arc},
};

use futures_util::{SinkExt, StreamExt};
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    stream::DuplexStream,
};

use log::*;
use rocket_ws::result::Result;
use thiserror::Error;
use tokio_tungstenite::tungstenite;

use super::{
    usernamemgr::{Key, NameLease},
    Chat,
};

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: Arc<str>,
    pub sender_id: u16,
    pub content: Arc<str>,
}
impl Message {
    pub const USERID_SPECIAL: u16 = 0;
    pub const SUBID_SETUP: u8 = 0;
    pub const SUBID_USERJOIN: u8 = 1;
    pub fn is_valid(&self) -> bool {
        if self.content.as_bytes().len() > 100 {
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
    pub fn new_setup<'a, 'b>(
        key: Key,
        id: u16,
        clients: Vec<ClientInfo>,
        history: Vec<Message>,
    ) -> tokio_tungstenite::tungstenite::Message {
        let key_str = key.to_string();
        let key_str_bytes = key_str.as_bytes();
        let mut data = Vec::with_capacity(key_str_bytes.len() + 3);
        data.extend_from_slice(&Self::USERID_SPECIAL.to_be_bytes());
        data.push(Self::SUBID_SETUP);
        data.extend_from_slice(&id.to_be_bytes());
        data.extend_from_slice(key_str_bytes);

        for client in clients {
            let name_bytes = client.username.as_bytes();
            data.reserve(name_bytes.len() + 3);
            data.extend_from_slice(&client.id.to_be_bytes());
            data.push(name_bytes.len() as u8);
            data.extend_from_slice(name_bytes);
        }
        data.extend_from_slice(&Self::USERID_SPECIAL.to_be_bytes());
        for message in history {
            let sender_bytes = message.sender.as_bytes();
            let content_bytes = message.content.as_bytes();
            data.reserve(sender_bytes.len() + content_bytes.len() + 2);
            data.push(sender_bytes.len() as u8);
            data.extend_from_slice(sender_bytes);
            data.push(content_bytes.len() as u8);
            data.extend_from_slice(content_bytes);
        }
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
        data.extend_from_slice(&mesg.sender_id.to_be_bytes());
        data.extend_from_slice(content_bytes);
        tungstenite::Message::Binary(data)
    }
}

pub struct ClientFactory {
    id_counter: AtomicU16,
}
impl ClientFactory {
    pub fn new() -> Self {
        Self {
            id_counter: 1.into(),
        }
    }
    pub fn reserve_id(&self) -> u16 {
        let value = self
            .id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if value == 0 {
            self.id_counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        } else {
            value
        }
    }
    pub async fn new_client(
        &self,
        mut ws: DuplexStream,
        key: Key,
        username: NameLease,
        chat_state: &Chat,
    ) -> Result<Client> {
        let id = self.reserve_id();
        ws.send(Message::new_setup(
            key,
            id,
            chat_state.clients().await,
            chat_state.history().await,
        ))
        .await?;
        let info = ClientInfo {
            username: username.into(),
            id,
        };
        let left_sender = chat_state.left_sender();
        Ok(Client {
            ws,
            info,
            left_sender,
        })
    }
}

#[derive(Debug, Error)]
pub enum PacketError {
    #[error("Invalid packet or protocol error")]
    Invalid,
    #[error("Client Disconnected")]
    Disconected,
}
impl From<tungstenite::Error> for PacketError {
    fn from(err: tungstenite::Error) -> Self {
        match err {
            tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
                PacketError::Disconected
            }
            _ => PacketError::Invalid,
        }
    }
}
pub struct Client {
    ws: DuplexStream,
    info: ClientInfo,
    left_sender: rocket::tokio::sync::broadcast::Sender<ClientInfo>,
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
    pub async fn try_recv(&mut self) -> Result<Option<Message>> {
        let Some(message) = self.ws.next().await else {
            return Ok(None);
        };
        let message = message?;
        if message.is_close() {
            return Ok(None);
        }
        if !message.is_text() {
            error!("Closing connection because: Received non text message");
            self.ws
                .close(Some(CloseFrame {
                    code: CloseCode::Unsupported,
                    reason: Cow::Borrowed("No non text messages."),
                }))
                .await?;
            return Ok(None);
        }
        let content = String::from_utf8_lossy(&message.into_data()).to_string();
        Ok(Some(Message {
            sender_id: self.info.id(),
            sender: self.info.username.clone(),
            content: content.into(),
        }))
    }

    pub fn client_info(&self) -> ClientInfo {
        self.info.clone()
    }

    pub async fn ratelimit_kick(&mut self) -> Result<()> {
        self.ws
            .close(Some(CloseFrame {
                code: rocket_ws::frame::CloseCode::Error,
                reason: Cow::Borrowed("ratelimit exceeded. Type a bit slower next time"),
            }))
            .await?;
        Ok(())
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        match self.left_sender.send(self.client_info()) {
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
