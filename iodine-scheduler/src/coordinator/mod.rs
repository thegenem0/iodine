use std::{
    pin::Pin,
    task::{Context, Poll},
};

use iodine_common::error::Error;
use tokio::{
    sync::{mpsc, oneshot},
    task::{JoinError, JoinHandle},
};
use uuid::Uuid;

use crate::launcher::LauncherCommand;

pub mod default;

pub struct CoordinatorConfig {
    pub max_launchers: usize,
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

pub struct ManagedLauncher {
    id: Uuid,
    command_tx: mpsc::Sender<LauncherCommand>,
    current_pipeline_id: Option<Uuid>,
}

pub struct MonitoredLauncherTask {
    launcher_id: Uuid,
    handle: JoinHandle<Result<(), Error>>,
}

impl Future for MonitoredLauncherTask {
    type Output = (Uuid, Result<Result<(), Error>, JoinError>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.handle).poll(cx) {
            Poll::Ready(join_result) => Poll::Ready((self.launcher_id, join_result)),
            Poll::Pending => Poll::Pending,
        }
    }
}
