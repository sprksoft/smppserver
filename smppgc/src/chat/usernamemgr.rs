use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Display,
    ops::Deref,
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime},
};

use uuid::Uuid;
struct NameSlot {
    name: Rc<str>,
    last_used: u64,
    owner: Key,
}
impl NameSlot {
    pub fn new(owner: Key, name: Rc<str>) -> Self {
        Self {
            last_used: Self::epoch(SystemTime::now()),
            owner,
            name,
        }
    }
    fn epoch(now: SystemTime) -> u64 {
        now.duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }
    pub fn lease(&mut self, name: Arc<str>, key: Key, reserve_time: u64) -> Option<NameLease> {
        if name.as_ref() != self.name.as_ref() {
            return None;
        }
        let now = SystemTime::now();

        let last_used_time = SystemTime::UNIX_EPOCH + Duration::from_secs(self.last_used);
        let age = now.duration_since(last_used_time).unwrap().as_secs();
        if self.owner == key || age > reserve_time {
            self.last_used = Self::epoch(now);
            self.owner = key;
            return Some(NameLease(name));
        }
        None
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
    names_to_slot: HashMap<Rc<str>, usize>,
    leased_slots: HashMap<Key, Vec<usize>>,
    slots: Vec<NameSlot>,
    reserve_time: u64,
}
impl UsernameManager {
    pub fn new(reserve_time: u64) -> Self {
        Self {
            reserve_time,
            names_to_slot: HashMap::new(),
            leased_slots: HashMap::new(),
            slots: Vec::new(),
        }
    }

    pub fn lease_name<'a>(
        &mut self,
        name: Arc<str>,
        key: Key,
    ) -> Result<NameLease, NameLeaseError> {
        if name.as_ref() == "system" {
            return Err(NameLeaseError::Taken);
        }
        let Some(name) = Self::validate_normalize_username(&name) else {
            return Err(NameLeaseError::Invalid);
        };
        let name: Arc<str> = name.into();

        if let Some(slot_index) = self.names_to_slot.get(name.as_ref()) {
            let slot = self.slots.get_mut(*slot_index).unwrap();
            if let Some(lease) = slot.lease(name, key, self.reserve_time) {
                return Ok(lease);
            }
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

#[derive(Eq, PartialEq, Clone, Hash)]
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
