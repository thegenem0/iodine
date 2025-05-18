use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: Uuid,
    pub pipeline_def_id: Uuid,
    pub status: PipelineRunStatus,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum PipelineRunStatus {
    Pending,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelling,
    Cancelled,
}
