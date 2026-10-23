use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// In-memory and Valkey compatible idempotency state tracker
#[derive(Clone, Default)]
pub struct IdempotencyStore {
    // In-memory fallback for local dev / tests
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check_and_set(&self, key: &str) -> bool {
        let mut map = self.cache.lock().await;
        if map.contains_key(key) {
            false
        } else {
            map.insert(key.to_string(), "PENDING".to_string());
            true
        }
    }

    pub async fn store_response(&self, key: &str, response: &str) {
        let mut map = self.cache.lock().await;
        map.insert(key.to_string(), response.to_string());
    }
}
