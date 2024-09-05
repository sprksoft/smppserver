use rocket_db_pools::sqlx;
use sqlx::PgConnection;
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

use crate::db::{self, Db};

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

    pub async fn lease_name(
        &self,
        name: &str,
        user_id: UserId,
        db: &mut PgConnection,
    ) -> Result<NameLease, NameLeaseError> {
        let Some(name) = Self::validate_normalize_username(name) else {
            return Err(NameLeaseError::Invalid);
        };
        let name = sqlx::query!(
            "WITH inserted AS (
                INSERT INTO name_links (name, owner, created_at) VALUES ($2, $1, extract(epoch from now())) ON CONFLICT (name) DO UPDATE SET created_at = extract(epoch from now()) WHERE name_links.owner = $1 AND name_links.name=$2
                RETURNING CASE
                    WHEN name_links.owner = $1 THEN name_links.name
                    ELSE NULL
                END
            ), removed AS (
                DELETE FROM name_links WHERE name NOT IN (SELECT name FROM name_links WHERE owner=$1 ORDER BY created_at DESC LIMIT $3) AND owner=$1
                ) SELECT * FROM inserted",
            user_id.uuid(),
            &name,
            self.max_reserved as i64
        )
        .fetch_optional(db)
        .await?.map(|name|name.case).flatten();

        match name {
            Some(name) => Ok(NameLease(name.into())),
            None => Err(NameLeaseError::Taken),
        }
    }

    fn is_valid_name_char(char: char) -> bool {
        if char.is_ascii() && !char.is_control() && char != '@' {
            true
        } else {
            false
        }
    }

    fn validate_normalize_username<'a>(name: &'a str) -> Option<Cow<'a, str>> {
        let name: &str = name.trim();
        if name.len() > 20 || name.len() < 2 {
            return None;
        }

        let mut new_name = None;
        for char in name.chars() {
            if !Self::is_valid_name_char(char) {
                return None;
            }
            if char.is_uppercase() {
                let mut new_name_string = String::with_capacity(name.len());
                for char in name.chars() {
                    if !Self::is_valid_name_char(char) {
                        return None;
                    }
                    if char == 'I' {
                        new_name_string.push('l');
                    } else {
                        for char in char.to_lowercase() {
                            new_name_string.push(char);
                        }
                    }
                }
                new_name = Some(new_name_string);
                break;
            }
        }
        match new_name {
            Some(name) => Some(Cow::Owned(name)),
            None => Some(Cow::Borrowed(name)),
        }
    }
}

#[derive(Error, Debug)]
pub enum NameLeaseError {
    #[error("Gebruikersnaam is ongeldig.")]
    Invalid,
    #[error("Gebruikersnaam is bezet.")]
    Taken,
    #[error("INT: db error")]
    Db(#[from] sqlx::Error),
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
