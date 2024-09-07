use tokio_tungstenite::tungstenite;

use super::{
    client::{ClientInfo, Message},
    usernamemgr::UserId,
};

pub const USERID_SPECIAL: u16 = 0;
pub const SUBID_SETUP: u8 = 0;
pub const SUBID_USERJOIN: u8 = 1;

pub fn new_setup<'a, 'b>(
    key: UserId,
    id: u16,
    clients: Vec<ClientInfo>,
    history: Vec<Message>,
) -> tokio_tungstenite::tungstenite::Message {
    let key_str = key.to_string();
    let key_str_bytes = key_str.as_bytes();
    let mut data = Vec::with_capacity(key_str_bytes.len() + 3);
    data.extend_from_slice(&USERID_SPECIAL.to_be_bytes());
    data.push(SUBID_SETUP);
    data.extend_from_slice(&id.to_be_bytes());
    data.extend_from_slice(key_str_bytes);

    for client in clients {
        let name_bytes = client.username().as_bytes();
        data.reserve(name_bytes.len() + 3);
        data.extend_from_slice(&client.id().to_be_bytes());
        data.push(name_bytes.len() as u8);
        data.extend_from_slice(name_bytes);
    }
    data.extend_from_slice(&USERID_SPECIAL.to_be_bytes());
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
    let username_bytes = client.username().as_bytes();
    let mut data = Vec::with_capacity(username_bytes.len() + 5);
    data.extend_from_slice(&USERID_SPECIAL.to_be_bytes());
    data.push(SUBID_USERJOIN);
    data.extend_from_slice(&client.id().to_be_bytes());
    data.extend_from_slice(&username_bytes);
    tokio_tungstenite::tungstenite::Message::Binary(data)
}
pub fn new_message(mesg: &Message) -> tokio_tungstenite::tungstenite::Message {
    new_seq_message(mesg, 0)
}
pub fn new_seq_message(mesg: &Message, seq_id: u8) -> tokio_tungstenite::tungstenite::Message {
    //|  u16 | local sender id
    //|  u8  | seq_id (incremented on every messages to this client)
    //|  u24 | time (minutes since UNIX_EPOCH)
    //| [u8] | content bytes

    let content_bytes = mesg.content.as_bytes();
    let mut data = Vec::with_capacity(content_bytes.len() + 2 + size_of::<u32>());
    data.extend_from_slice(&mesg.sender_id.to_be_bytes());
    data.push(seq_id);
    data.extend_from_slice(&mesg.timestamp.to_be_bytes()[1..]);
    data.extend_from_slice(content_bytes);
    tungstenite::Message::Binary(data)
}
