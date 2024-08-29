use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Display,
    ops::Deref,
    sync::Arc,
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

pub struct NameLease(Arc<str>);
impl Deref for NameLease {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<Arc<str>> for NameLease {
    fn into(self) -> Arc<str> {
        self.0.into()
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
    pub fn lease_name<'a>(&mut self, name: &'a str, key: Key) -> Result<NameLease, NameLeaseError> {
        if name == "system" {
            return Err(NameLeaseError::Taken);
        }
        let Some(name) = Self::validate_normalize_username(name) else {
            return Err(NameLeaseError::Invalid);
        };
        if let Some(slot) = self.names.get_mut(name.as_ref()) {
            let now = SystemTime::now();
            if slot.owner == key {
                slot.lease(key);
                return Ok(NameLease(name.into()));
            }
            let last_used_time = SystemTime::UNIX_EPOCH + Duration::from_secs(slot.last_used);
            let age = now.duration_since(last_used_time).unwrap().as_secs();
            if age > self.reserve_time {
                slot.lease(key);
                return Ok(NameLease(name.into()));
            }
        } else {
            self.names.insert(name.to_string(), NameSlot::new(key));
            return Ok(NameLease(name.into()));
        }

        Err(NameLeaseError::Taken)
    }

    fn validate_normalize_username<'a>(name: &'a str) -> Option<Cow<'a, str>> {
        if name.len() > 20 || name.len() < 2 {
            return None;
        }

        let mut new_name = String::with_capacity(name.len());
        for char in name.chars() {
            if !char.is_ascii() || char.is_control() || char == '@' {
                return None;
            }

            if char == 'I' {
                new_name.push('l');
            } else {
                for char in char.to_lowercase() {
                    new_name.push(char);
                }
            }
        }
        Some(Cow::Owned(new_name))
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
