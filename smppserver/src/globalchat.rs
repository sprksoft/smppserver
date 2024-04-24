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
use uuid::Uuid;

pub const SERVER_KEY: Uuid = Uuid::from_u128(0);

#[derive(Clone)]
pub struct ChatMessage {
    sender: Uuid,
    string: String,
}

#[derive(Responder)]
pub enum ChatResponder<O> {
    #[response(status = 418)]
    TeaPot(String),
    #[response(status = 400)]
    BadRequest(String),
    #[response(status = 401)]
    Unauthorized(String),
    #[response(status = 200)]
    Ok(O),
}
impl<O> ChatResponder<O> {
    pub fn invalid_key() -> Self {
        ChatResponder::Unauthorized("Invalid key".to_string())
    }
    pub fn too_long() -> Self {
        ChatResponder::BadRequest("Too long".to_string())
    }
    pub fn invalid_char() -> Self {
        ChatResponder::BadRequest("Invalid chars".to_string())
    }

    pub fn teapot() -> Self {
        ChatResponder::TeaPot("I am a teapot. Leave me alone please :(".to_string())
    }
}

#[get("/<key>")]
async fn chat(
    key: &str,
    queue: &State<Sender<ChatMessage>>,
    mut end: Shutdown,
) -> ChatResponder<EventStream![]> {
    let Ok(key) = Uuid::parse_str(key) else {
        return ChatResponder::invalid_key();
    };

    let mut rx = queue.subscribe();
    ChatResponder::Ok(EventStream! {
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

        }
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

#[post("/newkey")]
fn new_key() -> String {
    let key = Uuid::new_v4();
    key.simple().to_string()
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("globalchat", |rocket| async {
        rocket
            .manage(channel::<ChatMessage>(1024).0)
            .mount("/smpp/chat", routes![chat, send, new_key])
    })
}
