use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::{TaskDefinition, TaskDependency};

use super::{PipelineRun, PipelineRunConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineInfo {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub run_config: Option<PipelineRunConfig>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineDefinition {
    pub run_info: PipelineRun,
    pub nodes: Vec<TaskDefinition>,
    pub dependencies: Vec<TaskDependency>,
}
