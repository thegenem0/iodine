use std::{sync::Arc, time::Duration};

use iodine_common::{
    pipeline::PipelineDefinition, resource_manager::ProvisionedWorkerDetails, task::TaskRunStatus,
};
use tokio::sync::oneshot;
use uuid::Uuid;

pub mod default;
pub mod exec_mgr_registry;
pub mod execution_graph;
pub mod scheduling;
pub mod state;
pub mod util;

pub struct LauncherConfig {
    pub config: serde_json::Value,
    pub scheduler_tick_interval: Option<Duration>,
    pub worker_polling_interval: Option<Duration>,
    pub worker_channel_buffer_size: Option<usize>,
}

pub enum LauncherStatus {
    Initializing,
    Running,
    Terminating,
    Terminated,
}

#[non_exhaustive]
pub enum LauncherCommand {
    ExecutePipeline {
        pipeline_definition: Arc<PipelineDefinition>,
    },
    Terminate {
        ack_chan: oneshot::Sender<bool>,
    },
}

#[derive(Debug)]
pub struct WorkerResult {
    // The ID Launcher assigned to this execution attempt (matches WorkerRequest.worker_id)
    pub assigned_worker_id: Uuid,
    pub task_id: Uuid,
    pub attempt: u32,
    pub final_status: TaskRunStatus,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct ActiveWorkerInfo {
    task_id: Uuid,
    attempt: u32,
    provisioned_details: ProvisionedWorkerDetails,
    monitor_cancel_tx: Option<oneshot::Sender<()>>,
    task_name: String,
}
