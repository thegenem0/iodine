use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: Uuid,
    pub run_id: Uuid,
    pub task_def_id: Uuid,
    pub status: TaskRunStatus,
    pub attempts: i32,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub output_metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum TaskRunStatus {
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
