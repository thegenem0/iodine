use async_trait::async_trait;
use iodine_common::{
    error::Error,
    event::{EventSeverity, EventType},
    state::BaseDbTrait,
};
use uuid::Uuid;

use crate::{db::PostgresStateDb, event_logging::log_event_direct};

#[async_trait]
#[allow(unused_variables)]
impl BaseDbTrait for PostgresStateDb {
    async fn log_system_event(
        &self,
        run_id: Option<Uuid>,
        task_id: Option<Uuid>,
        message: String,
        metadata: Option<serde_json::Value>,
        severity: EventSeverity,
    ) -> Result<(), Error> {
        let mut event_meta = metadata
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()))
            .as_object()
            .cloned()
            .unwrap_or_default();

        event_meta.insert(
            "severity".to_string(),
            serde_json::Value::String(severity.to_string()),
        );

        log_event_direct(
            &self.conn,
            run_id,
            task_id,
            EventType::EngineEvent,
            Some(message),
            Some(serde_json::Value::Object(event_meta)),
        )
        .await
    }
}
