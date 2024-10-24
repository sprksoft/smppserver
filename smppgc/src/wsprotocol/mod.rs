use std::{
    borrow::Cow,
    time::{Duration, SystemTime},
};

use futures_util::SinkExt;
use rocket_ws::{
    frame::{CloseCode, CloseFrame},
    result::Result,
    stream::DuplexStream,
};
use thiserror::Error;
use tokio_tungstenite::tungstenite;

use crate::{chat::Message, names::UserId, userinfo::UserInfo};

use log::*;

mod packets;

#[derive(Debug, Error)]
pub enum PacketsError {
    #[error("Invalid packets or protocol error")]
    Invalid,
    #[error("Client Disconnected")]
    Disconected,
}
impl From<tungstenite::Error> for PacketsError {
    fn from(err: tungstenite::Error) -> Self {
        match err {
            tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
                PacketsError::Disconected
            }
            _ => PacketsError::Invalid,
        }
    }
}

#[derive(Clone, Copy)]
pub enum KickReason {
    RateLimit,
    ChatFull,
    Spam,
}
impl KickReason {
    pub fn into_close_frame(self) -> CloseFrame<'static> {
        match self {
            Self::RateLimit => CloseFrame {
                code: CloseCode::Error,
                reason: Cow::Borrowed("Te veel berichten. Typ de volgende keer wat langzamer."),
            },
            Self::Spam => CloseFrame {
                code: CloseCode::Error,
                reason: Cow::Borrowed("Please niet spammen"),
            },
            Self::ChatFull => CloseFrame {
                code: CloseCode::Again,
                reason: Cow::Borrowed("Chat zit vol."),
            },
        }
    }
}

pub struct WsClient {
    ws: DuplexStream,
    user_info: UserInfo,
}
impl WsClient {
    pub async fn new(
        mut ws: DuplexStream,
        user_static_id: UserId,
        user_info: UserInfo,
        clients: Vec<UserInfo>,
        history: Vec<Message>,
    ) -> Result<Self> {
        ws.send(packets::new_setup(
            user_static_id,
            user_info.id(),
            clients,
            history,
        ))
        .await?;

        Ok(Self { ws, user_info })
    }

    pub async fn kick(&mut self, reason: KickReason) -> Result<()> {
        self.ws.close(Some(reason.into_close_frame())).await
    }

    pub async fn forward_client(&mut self, client: &UserInfo) -> Result<()> {
        self.ws.send(packets::new_client_joined(client)).await?;
        Ok(())
    }
    pub async fn forward_all_clients(
        &mut self,
        clients: impl Iterator<Item = &UserInfo>,
    ) -> Result<()> {
        for client in clients {
            self.ws.feed(packets::new_client_joined(client)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn forward(&mut self, mesg: &Message) -> Result<()> {
        self.ws.send(packets::new_message(mesg)).await?;
        Ok(())
    }
    pub async fn forward_all(&mut self, messages: impl Iterator<Item = &Message>) -> Result<()> {
        for message in messages {
            self.ws.feed(packets::new_message(message)).await?;
        }
        self.ws.flush().await?;
        Ok(())
    }
    pub async fn try_recv(&mut self) -> Result<Option<Message>> {
        let Some(message) = futures_util::StreamExt::next(&mut self.ws).await else {
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
            sender_id: self.user_info.id(),
            sender: self.user_info.username.clone(),
            content: content.into(),
        }))
    }
}
