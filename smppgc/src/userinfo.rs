use std::sync::Arc;

#[derive(Clone, Debug, Hash)]
pub struct UserInfo {
    pub username: Arc<str>,
    pub id: u16,
}
impl Eq for UserInfo {}
impl UserInfo {
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn username(&self) -> &str {
        &self.username
    }
}
impl PartialEq for UserInfo {
    fn eq(&self, other: &Self) -> bool {
        other.id == self.id
    }
    fn ne(&self, other: &Self) -> bool {
        other.id != self.id
    }
}
