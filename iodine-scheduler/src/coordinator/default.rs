use std::{collections::HashMap, sync::Arc};

use futures::{StreamExt, stream::FuturesUnordered};
use iodine_common::{error::Error, state::DatabaseTrait};
use tokio::sync::{Mutex, RwLock, mpsc};
use uuid::Uuid;

use crate::launcher::{LauncherConfig, LauncherStatus, default::Launcher};

use super::{CoordinatorCommand, CoordinatorConfig, ManagedLauncher, MonitoredLauncherTask};

pub struct Coordinator {
    pub id: Uuid,
    pub config: CoordinatorConfig,
    pub state_manager: Arc<dyn DatabaseTrait>,
    active_launchers: Arc<RwLock<HashMap<Uuid, ManagedLauncher>>>,
    active_launcher_tasks: FuturesUnordered<MonitoredLauncherTask>,
    launcher_status_rx: Arc<Mutex<mpsc::Receiver<LauncherStatus>>>,
    launcher_status_tx_template: mpsc::Sender<LauncherStatus>, // Cloned for each new RunLauncher
    api_command_rx: mpsc::Receiver<CoordinatorCommand>,
}

impl Coordinator {
    pub fn new(
        config: CoordinatorConfig,
        state_manager: Arc<dyn DatabaseTrait>,
        api_command_rx: mpsc::Receiver<CoordinatorCommand>,
    ) -> Self {
        let (status_tx, status_rx) = mpsc::channel(config.max_launchers * 2);
        Self {
            id: Uuid::new_v4(),
            config,
            state_manager,
            active_launchers: Arc::new(RwLock::new(HashMap::new())),
            active_launcher_tasks: FuturesUnordered::new(),
            launcher_status_rx: Arc::new(Mutex::new(status_rx)),
            launcher_status_tx_template: status_tx.clone(),
            api_command_rx,
        }
    }

    #[allow(unused_variables)]
    async fn run_main_loop(&mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
                Some(command) = self.api_command_rx.recv() => {
                    match command {
                        CoordinatorCommand::SubmitPipeline { pipeline_id, run_id_override, response_oneshot } => {
                            if self.active_launchers.read().await.len() >= self.config.max_launchers {
                                eprintln!("Max launchers reached ({}), cannot submit pipeline {}", self.config.max_launchers, pipeline_id);
                                let _ = response_oneshot.send(Err(Error::Internal("Max launchers reached".to_string())));
                                continue;

                            }

                            let launcher_id_res = self.spawn_new_run_launcher().await;

                            let launcher_id = match launcher_id_res {
                                Ok(launcher_id) => launcher_id,
                                Err(err) => {
                                    let _ = response_oneshot.send(Err(Error::Internal(err.to_string())));
                                    continue;
                                }
                            };

                            let assignment_res = self.assign_pipeline_to_launcher(launcher_id, pipeline_id, run_id_override).await;

                            let pipeline_id = match assignment_res {
                                Ok(()) => pipeline_id,
                                Err(err) => {
                                    let _ = response_oneshot.send(Err(Error::Internal(err.to_string())));
                                    continue;
                                }
                            };

                            let _ = response_oneshot.send(Ok(pipeline_id));
                        }
                        CoordinatorCommand::CancelPipelineRun { pipeline_id, run_id, response_oneshot } => unimplemented!(),
                    }
                },
                maybe_status_update = async {
                    // Acquire lock outside of if let, guard lives for the scope of this async block
                    let mut guard = self.launcher_status_rx.lock().await;

                    // The guard is held until recv() completes or the async block is dropped.
                    guard.recv().await
                } => {
                    if let Some(status) = maybe_status_update {
                        match status {
                            LauncherStatus::Initializing => todo!("Handle Initializing"),
                            LauncherStatus::Running => todo!("Handle Running"),
                            LauncherStatus::Terminating => todo!("Handle Terminating"),
                            LauncherStatus::Terminated => {
                                // Example: Log termination or clean up launcher resources
                                // println!("Launcher reported Terminated status: {:?}", status);
                                todo!("Handle Terminated");
                            }
                        }
                    } else {
                        // This else branch is hit if recv() returns None,
                        // which means the channel is closed (all Senders were dropped).
                        // This creates a big mess, and the coordinator should
                        // probably die and restart.
                        // When in a multi-node deployment, just let it die,
                        // raft will elect a new leader and restart the coordinator.
                        eprintln!("Launcher status channel was closed. This happened on the Coordinator side. Solar flare bit-flip???");
                        return Err(Error::Internal("Launcher status channel closed on Coordinator side".to_string()));
                    }
                },

                Some((launcher_id, join_outcome)) = self.active_launcher_tasks.next(), if !self.active_launcher_tasks.is_empty() => {
                    match join_outcome {
                        Ok(run_loop_result) => {
                            // Task completed execution (didn't panic).
                            match run_loop_result {
                                Ok(_) => {
                                    println!("Launcher task {} completed successfully.", launcher_id);
                                }
                                Err(e) => {
                                    eprintln!("Launcher task {} completed with an internal error: {}", launcher_id, e);
                                    // TODO(thegenem0): Trigger recovery/notification for this specific error
                                }
                            }
                        }
                        Err(join_error) => {
                            // Task panicked (JoinError).
                            eprintln!("Launcher task {} panicked: {:?}", launcher_id, join_error);
                            // TODO(thegenem0): Trigger recovery logic for the pipeline this launcher was handling.
                        }
                    }

                    // Remove the launcher from active tracking.
                    // The tokio task (and launcher) is now finished.
                    if let Some(removed_launcher) = self.active_launchers.write().await.remove(&launcher_id) {
                        println!("Removed launcher {} from active map.", launcher_id);
                        // TODO(thegenem0): Clean up resources for this launcher.
                        // update pipeline status, etc.
                        if let Some(pipeline_id) = removed_launcher.current_pipeline_id {
                            eprintln!("Launcher {} was handling pipeline {}. Consider updating pipeline status.", launcher_id, pipeline_id);
                            // self.state_manager.update_pipeline_run_status(pipeline_id, ... "Failed/Completed" ...).await;
                        }
                    } else {
                        eprintln!("Attempted to remove launcher {}, but it was not found in the active map. (Already removed or race condition?)", launcher_id);
                    }
                }
            }
        }
    }

    async fn spawn_new_run_launcher(&mut self) -> Result<Uuid, Error> {
        let launcher_id = Uuid::new_v4();
        let (command_tx, command_rx) = mpsc::channel(10);

        let state_mgr_clone = Arc::clone(&self.state_manager);
        let status_tx_clone = self.launcher_status_tx_template.clone();

        let launcher_task_handle = tokio::spawn(async move {
            let mut launcher = Launcher::new(
                launcher_id,
                LauncherConfig {
                    config: serde_json::Value::default(),
                },
                state_mgr_clone,
                command_rx,
                status_tx_clone,
            );

            let launcher_run_result = launcher.run_loop().await;

            if let Err(e) = &launcher_run_result {
                eprintln!("Launcher task failed: {}", e);
            }

            launcher_run_result
        });

        let monitored_task = MonitoredLauncherTask {
            launcher_id,
            handle: launcher_task_handle,
        };

        self.active_launcher_tasks.push(monitored_task);

        let managed_launcher = ManagedLauncher {
            id: launcher_id,
            command_tx,
            current_pipeline_id: None, // We don't have any pipelines assigned yet
        };

        self.active_launchers
            .write()
            .await
            .insert(launcher_id, managed_launcher);

        Ok(launcher_id)
    }

    async fn assign_pipeline_to_launcher(
        &self,
        _launcher_id: Uuid,
        pipeline_id: Uuid,
        _run_id_override: Option<Uuid>,
    ) -> Result<(), Error> {
        let pipeline_def = self
            .state_manager
            .get_pipeline_definition(pipeline_id)
            .await?;

        match pipeline_def {
            Some(_pipeline_def) => {
                // TODO(thegenem0):
                // Implement this db call

                // self.state_manager
                //     .assign_pipeline_to_launcher(pipeline_id, launcher_id)
                //     .await?;
            }
            None => {
                return Err(Error::NotFound {
                    resource_type: "PipelineDefinition".to_string(),
                    resource_id: pipeline_id.to_string(),
                });
            }
        }

        Ok(())
    }

    async fn recover_state_on_startup(&self) -> Result<(), Error> {
        todo!()
    }

    // pub async fn run_main_loop(&mut self)
    //     // This loop would:
    //     // 1. tokio::select! over:
    //     //    a. Receiving commands from `api_command_receiver` (e.g., SubmitDagRequest).
    //     //    b. Receiving status updates from `run_launcher_status_receiver`.
    //     //    c. Periodic tasks (e.g., checking health of RunLauncher tasks, re-queueing timed-out DAGs).
    //     // 2. When a SubmitDagRequest comes:
    //     //    a. Fetch DagDefinition from StateManager (IMDG).
    //     //    b. Decide if a new RunLauncher task needs to be spawned or if an idle one can be used.
    //     //    c. Spawn/send ExecuteDag command to the chosen RunLauncher's channel.
    //     //    d. Update StateManager with DagRunStatus::Preparing and assigned RunLauncherId.
    //     // 3. When a RunLauncherStatusUpdate comes:
    //     //    a. Update StateManager with the final DagRunStatus.
    //     //    b. Mark the RunLauncher task as idle or ready for new work.
    //     //    c. Potentially trigger dependent DAGs or notifications.
    //     // 4. Handle RunLauncher task failures (e.g., if a JoinHandle indicates a panic).
}
