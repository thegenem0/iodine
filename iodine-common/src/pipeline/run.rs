use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineRun {
    pub run_id: Uuid,
    pub definition_id: Uuid,
    pub status: PipelineRunStatus,

    /// Configuration for this run
    pub run_config: serde_json::Value,

    /// Tags for this run (e.g., {"user": "xyz", "env": "prod"})
    pub tags: Option<serde_json::Value>,

    /// How was it triggered? {type: "schedule/sensor/manual", name: "..."}
    pub trigger_info: Option<serde_json::Value>,

    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineRunConfig {
    pub default_backend: PipelineRunBackend,
    pub default_tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
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

#[derive(Clone, Debug, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum PipelineRunBackend {
    Local,
    Docker,
    // K8S,
    // MicroVM,
    // AwsEcs,
    // GcpCloudRun,
}
