use std::collections::HashMap;

use async_trait::async_trait;
use iodine_common::{
    error::Error,
    event::{EventLogRecord, EventSeverity, EventType},
    state::BaseDbTrait,
};
use sea_orm::EntityTrait;
use uuid::Uuid;

use crate::{
    db::PostgresStateDb,
    entities::event_log,
    event_logging::log_event_direct,
    mapping::{db_error_to_domain, event_log_to_domain},
};

#[async_trait]
#[allow(unused_variables)]
impl BaseDbTrait for PostgresStateDb {
    async fn get_event_log(&self, record_id: i64) -> Result<Option<EventLogRecord>, Error> {
        let maybe_model = event_log::Entity::find_by_id(record_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        match maybe_model.map(event_log_to_domain) {
            Some(model) => model.map(Some),
            None => Ok(None),
        }
    }

    async fn list_event_logs(
        &self,
        run_id: Option<Uuid>,
        task_id: Option<Uuid>,
        severity: Option<EventSeverity>,
        limit: Option<u64>,
    ) -> Result<HashMap<Uuid, EventLogRecord>, Error> {
        todo!()
    }

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
