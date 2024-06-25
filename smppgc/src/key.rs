use std::fmt::Display;

use uuid::Uuid;

pub struct Key {
    uuid: Uuid,
    anon: bool,
}
impl Key {
    pub fn new() -> Key {
        Self {
            uuid: Uuid::new_v4(),
            anon: true,
        }
    }
    pub fn parse_str(string: &str) -> Option<(Self, String)> {
        let anon = if string.starts_with('l') {
            false
        } else if string.starts_with('a') {
            true
        } else {
            return None;
        };
        let username = string.rfind('|');
        let uuid = Uuid::parse_str(&string[1..]).ok()?;
        Some((Self { uuid, anon }, username))
    }
    pub fn to_string(&self) -> String {
        self.uuid.as_simple().to_string()
    }
}
impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.uuid.as_simple().fmt(f)
    }
}
