use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDependency {
    pub pipeline_id: Uuid,
    pub source_task_definition_id: Uuid,
    pub source_output_name: String,
    pub target_task_definition_id: Uuid,
    pub target_input_name: String,
}
