use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::error::Error;

pub struct Coordinator {
    pub id: Uuid,
    pub hostname: String,
    pub host_pid: i32,
    pub is_leader: bool,
    pub status: CoordinatorStatus,
    pub version: String,
    pub last_heartbeat: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

pub enum CoordinatorCommand {
    SubmitPipeline {
        pipeline_id: Uuid,
        run_id_override: Option<Uuid>,
        /// Send `pipeline_id` or `Error` back to the caller
        response_oneshot: oneshot::Sender<Result<Uuid, Error>>,
    },
    CancelPipelineRun {
        pipeline_id: Uuid,
        run_id: Uuid,
        /// Send `Ok` or `Error` back to the caller
        response_oneshot: oneshot::Sender<Result<(), Error>>,
    },
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "UPPERCASE")]
pub enum CoordinatorStatus {
    Pending,
    Running,
    Terminating,
    Terminated,
}
