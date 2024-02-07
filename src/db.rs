use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::sync::Notify;

#[derive(Clone)]
pub struct Value {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}
/**
* Key-Value database that stores the data.
* It is protected via an arc that safe to pass across threads.
* Implements a mutex to protect the data from multithread access.
 Mutex is implemented on the db as ttls will be accessed from only one thread thus no race conditions are possible
*/

pub struct Store {
    pub entries: HashMap<String, Value>,
    pub ttls: BTreeSet<(Instant, String)>,
}

pub struct Shared {
    pub state: Mutex<Store>,
    pub bg_task: Notify,
}

#[derive(Clone)]
pub struct DB {
    pub db: Arc<Shared>,
}

impl DB {
    pub fn new() -> DB {
        let shared = Shared {
            bg_task: Notify::new(),
            state: Mutex::new(Store {
                entries: HashMap::new(),
                ttls: BTreeSet::new(),
            }),
        };

        let shared = Arc::new(shared);

        tokio::spawn(delete_entries(shared.clone()));

        DB { db: shared }
    }

    pub fn set(&mut self, key: String, value: Vec<u8>, ttl: Option<usize>) {
        let mut store = self.db.state.lock().unwrap();

        let expires_at = if let Some(ttl) = ttl {
            let expires_at = Instant::now() + Duration::from_secs(ttl as u64);
            store.ttls.insert((expires_at, key.clone()));
            self.db.bg_task.notify_one();
            Some(expires_at)
        } else {
            None
        };
        store.entries.insert(key, Value { value, expires_at });
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let store = self.db.state.lock().unwrap();

        let value = store.entries.get(key);
        if let Some(value) = value {
            return Some(value.value.clone());
        }
        None
    }

    pub fn delete(&mut self, key: &str) -> Option<Vec<u8>> {
        let mut store = self.db.state.lock().unwrap();

        let value = store.entries.remove(key);
        if let Some(value) = value {
            return Some(value.value);
        }
        None
    }
}

impl Default for DB {
    fn default() -> Self {
        Self::new()
    }
}

impl Shared {
    fn delete_entries(&self) -> Option<Instant> {
        let mut store = self.state.lock().unwrap();
        let now = Instant::now();

        let store = &mut *store;

        while let Some(ttl) = store.ttls.iter().next().cloned() {
            if ttl.0 > now {
                return Some(ttl.0);
            }

            store.ttls.remove(&ttl);
            store.entries.remove(&ttl.1);
        }

        None
    }
}

async fn delete_entries(shared: Arc<Shared>) {
    loop {
        // Task should run after some ttl or if it get's notified.

        match shared.delete_entries() {
            Some(ttl) => {
                tokio::select! {
                    _ = tokio::time::sleep_until(tokio::time::Instant::from_std(ttl.to_owned())) => {}
                    _ = shared.bg_task.notified() => {}
                }
            }
            None => shared.bg_task.notified().await,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn set_get() {
        let mut db = DB::new();
        let value = "value".as_bytes().to_vec();
        db.set("key".to_string(), value.clone(), None);

        let result_value = db.get("key").unwrap();

        assert_eq!(value, result_value);
    }

    #[tokio::test]
    #[should_panic]
    async fn get_nonexistent() {
        let db = DB::new();

        db.get("key").unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn delete() {
        let mut db = DB::new();
        let value = "value".as_bytes().to_vec();
        db.set("key".to_string(), value.clone(), None);

        db.delete("key");

        db.get("key").unwrap();
    }

    #[tokio::test]
    #[should_panic]

    async fn test_expiry() {
        let mut db = DB::new();
        let value = "value".as_bytes().to_vec();
        db.set("key".to_string(), value.clone(), Some(2));
        tokio::time::sleep(Duration::from_secs(3)).await;
        db.get("key").unwrap();
    }
}
