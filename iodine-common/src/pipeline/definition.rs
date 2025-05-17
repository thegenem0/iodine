use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

use crate::task::TaskDefinition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineDefinition {
    pub info: PipelineInfo,
    pub task_definitions: Vec<TaskDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PipelineInfo {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub default_backend: PipelineBackend,
    pub default_tags: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, EnumString, Display, Default)]
#[strum(serialize_all = "UPPERCASE")]
pub enum PipelineBackend {
    #[default]
    Local,
    Docker,
    // K8S,
    // MicroVM,
    // AwsEcs,
    // GcpCloudRun,
}
