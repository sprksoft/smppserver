use std::{
    collections::HashMap,
    fmt::Display,
    time::{Duration, SystemTime},
};

use uuid::Uuid;
struct NameSlot {
    last_used: u64,
    owner: Key,
}
impl NameSlot {
    pub fn new(owner: Key) -> Self {
        Self {
            last_used: Self::current_epoch(),
            owner,
        }
    }
    fn current_epoch() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }
    pub fn lease(&mut self, key: Key) {
        self.last_used = Self::current_epoch();
        self.owner = key;
    }
}
pub struct UsernameManager {
    names: HashMap<String, NameSlot>,
    reserve_time: u64,
}
impl UsernameManager {
    pub fn new(reserve_time: u64) -> Self {
        Self {
            reserve_time,
            names: HashMap::new(),
        }
    }
    pub fn lease_name(&mut self, name: &str, key: Key) -> Result<(), NameLeaseError> {
        if name == "system" {
            return Err(NameLeaseError::Taken);
        }
        if !Self::validate_username(name) {
            return Err(NameLeaseError::Invalid);
        }
        if let Some(slot) = self.names.get_mut(name) {
            let now = SystemTime::now();
            if slot.owner == key {
                slot.lease(key);
                return Ok(());
            }
            let last_used_time = SystemTime::UNIX_EPOCH + Duration::from_secs(slot.last_used);
            let age = now.duration_since(last_used_time).unwrap().as_secs();
            if age > self.reserve_time {
                slot.lease(key);
                return Ok(());
            }
        } else {
            self.names.insert(name.to_string(), NameSlot::new(key));
            return Ok(());
        }

        Err(NameLeaseError::Taken)
    }

    fn validate_username(name: &str) -> bool {
        if name.len() > 15 || name.len() < 2 {
            return false;
        }
        for char in name.chars() {
            if !char.is_ascii() || char.is_control() {
                return false;
            }
        }

        true
    }
}
pub enum NameLeaseError {
    Invalid,
    Taken,
}

#[derive(Eq, PartialEq, Clone)]
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
}
impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.anon {
            f.write_str("a")?;
        } else {
            f.write_str("l")?;
        }
        self.uuid.as_simple().fmt(f)
    }
}
