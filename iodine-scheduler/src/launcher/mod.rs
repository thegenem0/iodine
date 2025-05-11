use std::{collections::HashMap, sync::Arc};

use iodine_common::pipeline::PipelineDefinition;
use uuid::Uuid;

pub mod default;

pub struct LauncherConfig {
    pub config: serde_json::Value,
}

pub enum LauncherStatus {
    Initializing,
    Running,
    Terminating,
    Terminated,
}

pub struct PipelineExecutionGraph {
    pub sorted_tasks: Vec<Uuid>,
    pub dependencies: HashMap<Uuid, Vec<Uuid>>, // task -> list of tasks it depends on
    pub dependents: HashMap<Uuid, Vec<Uuid>>,   // task -> list of tasks that depend on it
}

#[non_exhaustive]
pub enum LauncherCommand {
    ExecutePipeline {
        pipeline_definition: Arc<PipelineDefinition>,
    },
    Terminate,
}
