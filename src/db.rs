use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct Value {
    value: Vec<u8>,
    _expires_at: Option<Instant>,
}

#[derive(Clone)]
pub struct DB {
    pub db: Arc<Mutex<HashMap<String, Value>>>,
}

impl DB {
    pub fn new() -> DB {
        DB {
            db: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn set(&mut self, key: String, value: Vec<u8>, ttl: Option<usize>) {
        let mut db = self.db.lock().unwrap();

        let mut expires_at = None;
        if let Some(ttl) = ttl {
            expires_at = Some(Instant::now() + Duration::from_millis(ttl as u64));
        }
        db.insert(
            key,
            Value {
                value,
                _expires_at: expires_at,
            },
        );
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let db = self.db.lock().unwrap();

        let value = db.get(key);
        if let Some(value) = value {
            return Some(value.value.clone());
        }
        None
    }

    pub fn delete(&mut self, key: &str) -> Option<Vec<u8>> {
        let mut db = self.db.lock().unwrap();

        let value = db.remove(key);
        if let Some(value) = value {
            return Some(value.value);
        }
        None
    }
}
