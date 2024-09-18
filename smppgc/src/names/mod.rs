use dashmap::DashMap;
use log::*;
use rocket::{fairing::AdHoc, serde::Deserialize};
use std::{collections::VecDeque, ops::Deref, sync::Arc};
use thiserror::Error;

mod userid;
pub use userid::*;

#[derive(Error, Debug)]
pub enum NameClaimError {
    #[error("Gebruikersnaam is ongeldig.")]
    Invalid,
    #[error("Gebruikersnaam is bezet.")]
    Taken,
}

struct NameSlot {
    name: Arc<str>,
    owner: Option<UserId>,
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash, Clone)]
struct NormName(String);

pub struct UsernameManager {
    max_reserved: u16,
    names: DashMap<NormName, NameSlot>,
    claims: DashMap<UserId, VecDeque<NormName>>,
}
impl UsernameManager {
    pub fn new(max_reserved: u16) -> Self {
        Self {
            max_reserved,
            claims: DashMap::default(),
            names: DashMap::default(),
        }
    }

    pub fn claim_name(&self, name: &str, user_id: UserId) -> Result<ClaimedName, NameClaimError> {
        let name: Arc<str> = name.into();
        let norm_name = Self::normalize_name(&name).ok_or(NameClaimError::Invalid)?;

        {
            let mut slot = self
                .names
                .entry(norm_name.clone())
                .or_insert_with(|| NameSlot {
                    owner: Some(user_id.clone()),
                    name: name.clone(),
                });
            if slot.owner.as_ref().map(|o| *o != user_id).unwrap_or(false) {
                return Err(NameClaimError::Taken);
            }
            slot.owner = Some(user_id.clone());
            slot.name = name.clone();
        }

        let mut claimed_names = self
            .claims
            .entry(user_id)
            .or_insert(VecDeque::with_capacity(self.max_reserved as usize));

        if claimed_names.len() == self.max_reserved as usize {
            if let Some(name) = claimed_names.pop_back() {
                if name != norm_name {
                    self.names.remove(&name);
                }
            }
        }
        claimed_names.push_front(norm_name);

        Ok(ClaimedName(name))
    }

    fn is_valid_name_char(char: char) -> bool {
        if char.is_ascii() && !char.is_control() && char != '@' {
            true
        } else {
            false
        }
    }

    fn normalize_name<'a>(name: &'a str) -> Option<NormName> {
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
        Some(NormName(new_name))
    }
}

pub struct ClaimedName(Arc<str>);
impl Deref for ClaimedName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<Arc<str>> for ClaimedName {
    fn into(self) -> Arc<str> {
        self.0.into()
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NameConfig {
    pub max_reserved_names: u16,
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("username manager", |r| async {
        let config = r
            .figment()
            .extract::<NameConfig>()
            .expect("No username config");
        r.manage(UsernameManager::new(config.max_reserved_names))
    })
}
