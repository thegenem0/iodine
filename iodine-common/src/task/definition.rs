use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub name: String,
    pub description: Option<String>,

    /// Task configuration schema
    pub config_schema: Option<serde_json::Value>,

    /// Metadata about the code implementing this task (e.g., function name, module)
    pub user_code_metadata: Option<serde_json::Value>,

    /// Task ids on which this task depends
    pub depends_on: Vec<Uuid>,
}
