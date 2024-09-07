use std::{
    borrow::Cow,
    cell::Cell,
    hash::Hash,
    sync::{atomic::AtomicU16, Arc},
    time::{Duration, SystemTime},
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
    joined_total, packet,
    usernamemgr::{NameLease, UserId},
    Chat,
};

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: Arc<str>,
    pub content: Arc<str>,
    pub timestamp: u32,
    pub sender_id: u16,
}
impl Message {
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
        key: UserId,
        username: NameLease,
        chat_state: &Chat,
    ) -> Result<Client> {
        let id = self.reserve_id();
        joined_total::inc();
        ws.send(packet::new_setup(
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
            seq_id: Cell::new(0),
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
    seq_id: Cell<u8>,
    ws: DuplexStream,
    info: ClientInfo,
    left_sender: rocket::tokio::sync::broadcast::Sender<ClientInfo>,
}
impl Client {
    pub async fn forward_client(&mut self, client: &ClientInfo) -> Result<()> {
        self.ws.send(packet::new_client_joined(client)).await?;
        Ok(())
    }
    pub async fn forward_all_clients(
        &mut self,
        clients: impl Iterator<Item = &ClientInfo>,
    ) -> Result<()> {
        for client in clients {
            self.ws.feed(packet::new_client_joined(client)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn forward(&mut self, mesg: &Message) -> Result<()> {
        let seq_id = self.seq_id.take();
        self.ws.send(packet::new_seq_message(mesg, seq_id)).await?;
        self.seq_id.set(seq_id);
        Ok(())
    }
    pub async fn forward_all(&mut self, messages: impl Iterator<Item = &Message>) -> Result<()> {
        let mut seq_id = self.seq_id.take();
        for message in messages {
            self.ws
                .feed(packet::new_seq_message(message, seq_id))
                .await?;
            seq_id += 1;
        }
        self.seq_id.set(seq_id);
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn try_recv(&mut self) -> Result<Option<Message>> {
        let Some(message) = self.ws.next().await else {
            return Err(rocket_ws::result::Error::ConnectionClosed);
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
                    reason: Cow::Borrowed("INT: No non text messages."),
                }))
                .await?;
            return Ok(None);
        }
        let content = String::from_utf8_lossy(&message.into_data()).to_string();

        let timestamp = (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| {
                error!("Time went backwards");
                Duration::from_secs(0)
            })
            .as_secs()
            / 60) as u32;

        Ok(Some(Message {
            timestamp,
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
                reason: Cow::Borrowed("Te veel berichten. Typ de volgende keer wat langzamer."),
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
impl ClientInfo {
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn username(&self) -> &str {
        &self.username
    }
}
impl PartialEq for ClientInfo {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
    fn ne(&self, other: &Self) -> bool {
        other.id != self.id
    }
}
