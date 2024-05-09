use std::{
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use log::*;
use rocket::{
    fairing::AdHoc,
    get, post,
    response::{
        self,
        stream::{Event, EventStream},
    },
    routes,
    tokio::{
        select,
        sync::broadcast::{channel, error::RecvError, Sender},
    },
    Responder, Shutdown, State,
};
use thiserror::Error;
use uuid::Uuid;

pub const SERVER_KEY: Uuid = Uuid::from_u128(0);
pub const MAX_KEY_COUNT: usize = 6000;

#[derive(Clone)]
pub struct ChatMessage {
    sender: Uuid,
    string: String,
}

pub struct Key {
    key: Uuid,
    time: u64,
}

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Lock poisoned")]
    LockPoisoned(),
    #[error("Max user count reached")]
    TooManyKeys(),
}
pub struct KeyManager {
    active_keys: Mutex<Vec<Key>>,
}
impl KeyManager {
    pub fn new_key_time(&self) -> Result<(Uuid, u64), KeyError> {
        let key = Uuid::new_v4();
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards (before epoch)")
            .as_secs();

        let mut lock = self.active_keys.lock().map_err(|_| {
            error!("mutex poisoned");
            KeyError::LockPoisoned()
        })?;
        if lock.len() >= MAX_KEY_COUNT {
            lock.retain(|key| (time - key.time < 10) || key.time == 0);
            if lock.len() >= MAX_KEY_COUNT {
                return Err(KeyError::TooManyKeys());
            }
        }
        lock.push(Key { key, time });
        Ok((key, time))
    }
    pub fn set_active(&self, key: Uuid, value: bool) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards (before epoch)")
            .as_secs();
        let lock = self.active_keys.lock().unwrap();
        for okey in lock.iter_mut() {
            if okey.key == key {
                if value {
                    okey.time = 0;
                } else {
                    okey.time = now;
                }
                return true;
            }
        }
        false
    }
}

#[derive(Responder)]
pub enum ChatResponder<O> {
    #[response(status = 418)]
    TeaPot(&'static str),
    #[response(status = 400)]
    BadRequest(&'static str),
    #[response(status = 401)]
    Unauthorized(&'static str),
    #[response(status = 200)]
    Ok(O),
}
impl<O> ChatResponder<O> {
    pub fn invalid_key() -> Self {
        ChatResponder::Unauthorized("Invalid key")
    }
    pub fn too_long() -> Self {
        ChatResponder::BadRequest("Too long")
    }
    pub fn invalid_char() -> Self {
        ChatResponder::BadRequest("Invalid chars")
    }

    pub fn teapot() -> Self {
        ChatResponder::TeaPot("I am a teapot. Leave me alone please :(")
    }
}

#[get("/<key>")]
async fn chat(
    key: &str,
    queue: &State<Sender<ChatMessage>>,
    km: &State<KeyManager>,
    mut end: Shutdown,
) -> ChatResponder<EventStream![]> {
    let Ok(key) = Uuid::parse_str(key) else {
        return ChatResponder::invalid_key();
    };
    if key == SERVER_KEY {
        return ChatResponder::invalid_key();
    }

    let mut rx = queue.subscribe();
    ChatResponder::Ok(EventStream! {
        km.set_active(key, true);
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };
            if msg.sender == key{
                yield Event::data(msg.string).event("y");
            }else if msg.sender == SERVER_KEY{
                yield Event::data(msg.string).event("s");
            }
            else{
                yield Event::data(msg.string).event("a");
            }

        };
        km.set_active(key, false);
    })
}

#[post("/<key>/send", data = "<data>")]
fn send(key: &str, queue: &State<Sender<ChatMessage>>, data: String) -> ChatResponder<String> {
    let Ok(key) = Uuid::parse_str(key) else {
        return ChatResponder::invalid_key();
    };
    if data.len() > 25 {
        return ChatResponder::too_long();
    }
    for character in data.chars() {
        if !character.is_ascii() {
            return ChatResponder::invalid_char();
        }
    }

    let _ = queue.send(ChatMessage {
        sender: key,
        string: data,
    });
    ChatResponder::Ok("ok".to_string())
}

#[derive(Responder)]
pub enum NewKeyResponse {
    #[response(status = 200)]
    Ok(String),
    #[response(status = 418)]
    TeaPot(&'static str),
    #[response(status = 429)]
    Badrequest(&'static str),
}
impl NewKeyResponse {
    pub fn teapot() -> Self {
        Self::TeaPot("I am a teapot. Leave me alone please :(")
    }
    pub fn ok(key: Uuid) -> Self {
        Self::Ok(key.simple().to_string())
    }
    pub fn room_filled() -> Self {
        Self::Badrequest("Too many people in chat right now")
    }
}

#[post("/newkey")]
fn new_key(km: &State<KeyManager>) -> Result<NewKeyResponse, rocket::response::Debug<KeyError>> {
    let (key, _) = match km.new_key_time() {
        Ok(val) => val,
        Err(err) => match err {
            KeyError::TooManyKeys() => return Ok(NewKeyResponse::room_filled()),
            _ => return Err(response::Debug(err)),
        },
    };
    Ok(NewKeyResponse::ok(key))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("globalchat", |rocket| async {
        rocket
            .manage(channel::<ChatMessage>(1024).0)
            .manage(KeyManager {
                active_keys: vec![].into(),
            })
            .mount("/smpp/chat", routes![chat, send, new_key])
    })
}
