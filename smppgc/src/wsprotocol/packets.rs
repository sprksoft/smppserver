use tokio_tungstenite::tungstenite;

use crate::{chat::Message, names::UserId, userinfo::UserInfo};

pub const USERID_SPECIAL: u16 = 0;
pub const SUBID_SETUP: u8 = 0;
pub const SUBID_USERJOIN: u8 = 1;

pub fn new_setup<'a, 'b>(
    key: UserId,
    id: u16,
    clients: Vec<UserInfo>,
    history: Vec<Message>,
) -> tokio_tungstenite::tungstenite::Message {
    //|    u16   | const USERID_SPECIAL
    //|    u8    | const SUBID_SETUP
    //|    u16   | id
    //| [u8; 33] | key
    //
    //  clients:
    //|    u16   | client count
    //|    u16   | client id
    //|    u8    | username len
    //|    [u8]  | username
    //
    //  hist messages:
    //|    u32   | time (minutes since UNIX_EPOCH)
    //|    u8    | sender username len
    //|    [u8]  | sender username
    //|    u8    | content len
    //|    [u8]  | content

    let key_str = key.to_string();
    let key_str_bytes = key_str.as_bytes();
    let mut data = Vec::with_capacity(2 + 1 + 2 + key_str_bytes.len());
    data.extend_from_slice(&USERID_SPECIAL.to_be_bytes());
    data.push(SUBID_SETUP);
    data.extend_from_slice(&id.to_be_bytes());
    data.extend_from_slice(key_str_bytes);

    data.extend_from_slice(&(clients.len() as u16).to_be_bytes());
    for client in clients {
        let name_bytes = client.username().as_bytes();
        data.reserve(name_bytes.len() + 3);
        data.extend_from_slice(&client.id().to_be_bytes());
        data.push(name_bytes.len() as u8);
        data.extend_from_slice(name_bytes);
    }
    for message in history {
        let sender_bytes = message.sender.as_bytes();
        let content_bytes = message.content.as_bytes();
        data.reserve(sender_bytes.len() + content_bytes.len() + 2 + 8);
        data.extend_from_slice(&message.timestamp.to_be_bytes());
        data.push(sender_bytes.len() as u8);
        data.extend_from_slice(sender_bytes);
        data.push(content_bytes.len() as u8);
        data.extend_from_slice(content_bytes);
    }
    tokio_tungstenite::tungstenite::Message::Binary(data)
}
pub fn new_client_joined(client: &UserInfo) -> tokio_tungstenite::tungstenite::Message {
    //|  u16 | const USERID_SPECIAL
    //|  u8  | const SUBID_USERJOIN
    //| u16  | user id
    //| [u8]  | username

    let username_bytes = client.username().as_bytes();
    let mut data = Vec::with_capacity(username_bytes.len() + 5);
    data.extend_from_slice(&USERID_SPECIAL.to_be_bytes());
    data.push(SUBID_USERJOIN);
    data.extend_from_slice(&client.id().to_be_bytes());
    data.extend_from_slice(&username_bytes);
    tokio_tungstenite::tungstenite::Message::Binary(data)
}
pub fn new_message(mesg: &Message) -> tokio_tungstenite::tungstenite::Message {
    //|  u16 | local sender id
    //|  u32 | time (minutes since UNIX_EPOCH)
    //| [u8] | content bytes

    let content_bytes = mesg.content.as_bytes();
    let mut data = Vec::with_capacity(content_bytes.len() + size_of::<u16>() + size_of::<u32>());
    data.extend_from_slice(&mesg.sender_id.to_be_bytes());
    data.extend_from_slice(&mesg.timestamp.to_be_bytes());
    data.extend_from_slice(content_bytes);
    tungstenite::Message::Binary(data)
}
