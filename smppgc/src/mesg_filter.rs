use crate::{chat::client::Message, ChatConfig};

pub enum Cmd {
    KillMe,
    BlockMe,
}

pub enum FilterResult {
    Message(Message),
    Cmd(Cmd),
    Invalid,
}

fn parse_cmd(str: &str) -> Option<Cmd> {
    if str == "/killme" {
        Some(Cmd::KillMe);
    } else if str == "/blockme" {
        Some(Cmd::BlockMe);
    }
    None
}

pub fn filter(mut mesg: Message, max_msg_len: usize) -> FilterResult {
    if !mesg.is_valid(max_msg_len) {
        return FilterResult::Invalid;
    };
    let content = mesg.content.as_ref().trim();
    if let Some(cmd) = parse_cmd(content) {
        return FilterResult::Cmd(cmd);
    }

    let word = ['k', 'y', 's'];
    let is_kys = content
        .chars()
        .zip(word)
        .filter(|(char, char2)| !char.is_whitespace() && !char2.is_whitespace())
        .all(|(char, word_char)| char.to_lowercase().next() == Some(word_char));

    if is_kys {
        mesg.content = "Kiss me pwees".into();
    }

    FilterResult::Message(mesg)
}

pub struct MesgFilter {}
impl MesgFilter {}
