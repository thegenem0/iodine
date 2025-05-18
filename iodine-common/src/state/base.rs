use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug};
use uuid::Uuid;

use crate::{
    error::Error,
    event::{EventLogRecord, EventSeverity},
};

#[async_trait]
pub trait BaseDbTrait: Send + Sync + Debug + 'static {
    /// Gets an event log record by `record_id`
    /// ---
    async fn get_event_log(&self, record_id: i64) -> Result<Option<EventLogRecord>, Error>;

    /// Lists event log records
    /// ---
    /// Can optionally filter by `run_id`, `task_id`, and `severity`
    /// DO NOT CALL with no arguments,
    /// as there may be millions of records.
    /// It is left open for customizability,
    /// but can hammer performance if not used carefully.
    /// TODO(thegenem0): Should this be paginated?
    async fn list_event_logs(
        &self,
        pipeline_run_id: Option<Uuid>,
        task_run_id: Option<Uuid>,
        severity: Option<EventSeverity>,
        limit: Option<u64>,
    ) -> Result<HashMap<Uuid, EventLogRecord>, Error>;

    /// Logs a generic system event not tied to specific state updates.
    /// ---
    /// Should be used sparingly.
    /// Logging should happen as part of every database call,
    /// as part of a transaction, and failures should be handled
    /// appropriately within the given database call.
    async fn log_system_event(
        &self,
        pipeline_run_id: Option<Uuid>,
        task_run_id: Option<Uuid>,
        message: String,
        metadata: Option<serde_json::Value>,
        severity: EventSeverity,
    ) -> Result<(), Error>;
}
