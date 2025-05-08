use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerError {
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
