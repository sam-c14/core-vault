use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    AccountCreated,
    BalanceUpdated,
    LienPlaced,
    LienReleased,
    AccountStatusChanged,
    TransferPosted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub actor_id: Uuid,
    pub action: AuditAction,
    pub entity_name: String,
    pub entity_id: Uuid,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn record(&self, event: AuditEvent) -> Result<(), String>;
}

#[derive(Clone, Default)]
pub struct MemoryAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl MemoryAuditSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_events(&self) -> Vec<AuditEvent> {
        self.events.lock().await.clone()
    }
}

#[async_trait]
impl AuditSink for MemoryAuditSink {
    async fn record(&self, event: AuditEvent) -> Result<(), String> {
        let mut lock = self.events.lock().await;
        lock.push(event);
        Ok(())
    }
}
