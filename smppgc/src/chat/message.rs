use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: Arc<str>,
    pub content: Arc<str>,
    pub timestamp: u32,
    pub sender_id: u16,
}
impl Message {
    pub fn is_valid(&self, max_len: usize) -> bool {
        if self.content.len() > max_len {
            return false;
        }
        if self.is_empty() {
            return false;
        }
        true
    }
    pub fn is_empty(&self) -> bool {
        for char in self.content.chars() {
            if !char.is_whitespace() {
                return false;
            }
        }
        true
    }
}
