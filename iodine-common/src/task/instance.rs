use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskInstance {
    pub id: Uuid,
    pub run_id: Uuid,
    pub definition_id: Uuid,
    pub name: String,
    pub status: TaskStatus,
    pub attempts: i32,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub worker_id: Option<Uuid>,
    pub output_metadata: Option<serde_json::Value>,
    pub error_data: Option<serde_json::Value>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum TaskStatus {
    Pending,
    Queued,
    Running,
    Retrying,
    Succeeded,
    Failed,
    Cancelling,
    Cancelled,
    Skipped,
}
