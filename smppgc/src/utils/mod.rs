use std::sync::atomic::AtomicU16;

pub mod dropvec;
pub mod static_routing;

pub struct IdCounter {
    id_counter: AtomicU16,
}
impl IdCounter {
    pub fn new() -> Self {
        Self {
            id_counter: 1.into(),
        }
    }
    pub fn new_id(&self) -> u16 {
        let value = self
            .id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if value == 0 {
            self.id_counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        } else {
            value
        }
    }
}
