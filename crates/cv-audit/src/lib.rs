pub mod logger;

pub use logger::{AuditAction, AuditEvent, AuditSink, MemoryAuditSink};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_memory_audit_sink_records_and_retrieves() {
        let sink = MemoryAuditSink::new();
        let event = AuditEvent {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            action: AuditAction::TransferPosted,
            entity_name: "Transaction".to_string(),
            entity_id: Uuid::new_v4(),
            payload: json!({
                "amount": "25000.00",
                "currency": "NGN"
            }),
            timestamp: Utc::now(),
        };

        sink.record(event.clone()).await.unwrap();
        let events = sink.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity_name, "Transaction");
    }
}

