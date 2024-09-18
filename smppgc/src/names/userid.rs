use std::fmt::Display;

use uuid::Uuid;

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct UserId {
    uuid: Uuid,
    anon: bool,
}
impl UserId {
    pub fn new() -> UserId {
        Self {
            uuid: Uuid::new_v4(),
            anon: true,
        }
    }
    pub fn parse_str(string: &str) -> Option<Self> {
        if string.len() != 33 {
            return None;
        }
        let anon = if string.starts_with('l') {
            false
        } else if string.starts_with('a') {
            true
        } else {
            return None;
        };
        let uuid = Uuid::parse_str(&string[1..]).ok()?;
        Some(Self { uuid, anon })
    }
    pub fn to_bytes_le(&self) -> [u8; 17] {
        let mut out = [0; 17];
        out.clone_from_slice(&self.uuid.to_bytes_le());
        if self.anon {
            out[16] = 0x61; //a
        } else {
            out[16] = 0x6C; //l
        }
        out
    }
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}
impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.anon {
            f.write_str("a")?;
        } else {
            f.write_str("l")?;
        }
        self.uuid.as_simple().fmt(f)
    }
}
