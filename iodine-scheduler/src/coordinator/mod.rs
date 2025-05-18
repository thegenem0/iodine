use std::{
    pin::Pin,
    task::{Context, Poll},
};

use iodine_common::error::Error;
use tokio::{
    sync::oneshot,
    task::{JoinError, JoinHandle},
};
use uuid::Uuid;

pub mod default;

pub struct CoordinatorConfig {
    pub max_launchers: usize,
}

#[non_exhaustive]
pub enum SupervisorCommand {
    Terminate { ack_chan: oneshot::Sender<bool> },
}

pub struct ManagedLauncher {
    id: Uuid,
    current_pipeline_id: Option<Uuid>,
    current_run_id: Option<Uuid>,
}

type LauncherRunResult = Result<(), Error>;

pub struct MonitoredLauncherTask {
    launcher_id: Uuid,
    handle: JoinHandle<LauncherRunResult>,
}

impl Future for MonitoredLauncherTask {
    type Output = (Uuid, Result<LauncherRunResult, JoinError>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.handle).poll(cx) {
            Poll::Ready(join_result) => Poll::Ready((self.launcher_id, join_result)),
            Poll::Pending => Poll::Pending,
        }
    }
}
