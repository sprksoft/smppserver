use std::{
    ops::AddAssign,
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub struct ProfFilter {
    censor: Arc<RwLock<censor::Censor>>,
}
impl ProfFilter {
    pub fn new() {
        Self {
            censor: RwLock::new(censor::Sex + censor::Zealous + censor::Standard).into(),
        };
    }

    pub fn load_wordlist(&self, path: &str) -> std::io::Result<()> {
        let mut lock = self.censor.write().unwrap();
        for line in std::fs::read_to_string(path)?.split('\n') {
            let line = line.trim();
            if line.len() == 0 || line.starts_with("#") {
                continue;
            }
            lock.add_assign(line);
        }
        Ok(())
    }
    pub fn filter(&self, string: &str) -> String {
        self.censor.read().unwrap().replace(string, "#")
    }
}
