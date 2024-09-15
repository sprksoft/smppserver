use log::*;
use rocket_db_pools::deadpool_redis::{
    redis::{
        self,
        aio::{ConnectionLike, MultiplexedConnection},
        pipe, AsyncCommands, ToRedisArgs,
    },
    Connection,
};
use std::{
    borrow::Cow,
    fmt::Display,
    ops::Deref,
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime},
};
use thiserror::Error;

use uuid::Uuid;

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
    max_reserved: u16,
}
impl UsernameManager {
    pub fn new(max_reserved: u16) -> Self {
        Self { max_reserved }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH+Duration::from_secs(60*))
            .unwrap_or_else(|_| {
                error!("Time went backwards");
                Duration::from_secs(0)
            })
            .as_millis() as u64
    }

    pub async fn lease_name(
        &self,
        name: &str,
        user_id: UserId,
        db: &mut MultiplexedConnection,
    ) -> Result<NameLease, NameLeaseError> {
        let Some(norm_name) = Self::normalize_name(name) else {
            return Err(NameLeaseError::Invalid);
        };

        let claimed_names_key = format!("claimed_names:{}", user_id);
        let wanted_name_key = format!("name_owners:{}", norm_name);

        let mut pipe = redis::pipe();
        pipe.atomic();
        for name in db
            .zrange::<_, String>(&claimed_names_key, (self.max_reserved) as isize, -1)
            .await
            .iter()
        {
            pipe.del(format!("name_owners:{}", name)).ignore();
        }
        pipe.zremrangebyrank(&claimed_names_key, self.max_reserved as isize, -1)
            .ignore();

        pipe.query_async(db).await?;
        let wanted_slot: Option<String> = db.get(&wanted_name_key).await?;
        let avail = wanted_slot
            .map(|id| id == user_id.to_string())
            .unwrap_or(true);

        if avail {
            pipe.set(&wanted_name_key, &user_id);
            pipe.zadd(&claimed_names_key, &norm_name, u64::MAX - Self::now());
            pipe.query_async(db).await?;
            Ok(NameLease(name.into()))
        } else {
            Err(NameLeaseError::Taken)
        }
    }

    fn is_valid_name_char(char: char) -> bool {
        if char.is_ascii() && !char.is_control() && char != '@' {
            true
        } else {
            false
        }
    }

    fn normalize_name<'a>(name: &'a str) -> Option<String> {
        let name: &str = name.trim();
        if name.len() > 20 || name.len() < 2 {
            return None;
        }

        let mut new_name = String::with_capacity(name.len());
        for char in name.chars() {
            if !Self::is_valid_name_char(char) {
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
        Some(new_name)
    }
}

#[derive(Error, Debug)]
pub enum NameLeaseError {
    #[error("Gebruikersnaam is ongeldig.")]
    Invalid,
    #[error("Gebruikersnaam is bezet.")]
    Taken,
    #[error("INT: db error")]
    Db(#[from] redis::RedisError),
}

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
impl ToRedisArgs for UserId {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg_fmt(self)
    }
}
