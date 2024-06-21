use std::net::TcpStream;

use log::*;
use tungstenite::WebSocket;

use crate::key::Key;

pub struct Message {
    pub sender: u16,
    pub content: String,
}
impl Message {
    pub fn is_valid(&self) -> bool {
        if self.content.as_bytes().len() > 25 {
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
    id_counter: u16,
}
impl ClientFactory {
    pub fn new() -> Self {
        Self { id_counter: 1 }
    }
    pub fn new_client(&mut self, mut ws: WebSocket<TcpStream>, key: Key) -> Client {
        self.id_counter += 1;
        let mut alive = ws.can_read() && ws.can_write();
        if alive {
            match ws.send(Self::setup_mesg(key, self.id_counter)) {
                Ok(()) => {}
                Err(err) => {
                    error!("Failed to send setup message: {}", err);
                    alive = false;
                }
            };
        }
        Client {
            alive,
            ws,
            username: "unnamed_user".to_string(),
            id: self.id_counter,
        }
    }
    fn setup_mesg(key: Key, id: u16) -> tungstenite::Message {
        let str = key.to_string();
        let str_bytes = str.as_bytes();
        let mut data = Vec::with_capacity(str_bytes.len() + 2);
        data.push(0);
        data.push(0);
        data.extend_from_slice(&id.to_be_bytes());
        data.extend_from_slice(str_bytes);
        tungstenite::Message::Binary(data)
    }
}

pub struct Client {
    ws: WebSocket<TcpStream>,
    username: String,
    id: u16,
    alive: bool,
}
impl Client {
    pub fn try_recv(&mut self) -> Option<Message> {
        match self.ws.read() {
            Ok(mesage) => {
                if !mesage.is_text() {
                    return None;
                }
                let content = String::from_utf8_lossy(&mesage.into_data()).to_string();
                Some(Message {
                    sender: self.id,
                    content,
                })
            }
            Err(err) => {
                self.handle_socket_error(err);
                None
            }
        }
    }
    pub fn send(&mut self, mesg: &Message) {
        if !self.alive() {
            return;
        }
        let content_bytes = mesg.content.as_bytes();
        let mut data = Vec::with_capacity(content_bytes.len() + 2);
        data.extend_from_slice(&mesg.sender.to_be_bytes());
        data.extend_from_slice(content_bytes);
        match self.ws.write(tungstenite::Message::Binary(data)) {
            Ok(_) => {}
            Err(err) => self.handle_socket_error(err),
        };
        match self.ws.flush() {
            Ok(()) => {}
            Err(err) => self.handle_socket_error(err),
        }
    }
    fn handle_socket_error(&mut self, err: tungstenite::Error) {
        match err {
            tungstenite::Error::ConnectionClosed => self.alive = false,
            err => error!("Socket error: {}", err),
        }
    }
    pub fn alive(&self) -> bool {
        self.alive
    }
    pub fn username(&self) -> &str {
        &self.username
    }
    pub fn id(&self) -> u16 {
        self.id
    }
}
