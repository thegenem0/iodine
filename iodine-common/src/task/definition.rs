use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::Error, resource_manager::ExecutionContext};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: Uuid,
    pub pipeline_def_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub execution_ctx: ExecutionContext,
    pub max_attempts: Option<i32>,

    /// Task ids on which this task depends
    pub depends_on: Vec<Uuid>,
}

impl TryFrom<TaskDefinition> for serde_json::Value {
    type Error = Error;

    fn try_from(value: TaskDefinition) -> Result<Self, Self::Error> {
        serde_json::to_value(value).map_err(Error::Serialization)
    }
}
