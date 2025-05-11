use std::{collections::HashMap, process, sync::Arc};

use futures::{StreamExt, stream::FuturesUnordered};
use iodine_common::{
    coordinator::{CoordinatorCommand, CoordinatorStatus},
    error::Error,
    state::DatabaseTrait,
};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use uuid::Uuid;

use crate::{
    launcher::{LauncherConfig, LauncherStatus, default::Launcher},
    resource_manager::default::ResourceManager,
};

use super::{CoordinatorConfig, ManagedLauncher, MonitoredLauncherTask, SupervisorCommand};

pub struct Coordinator {
    /// Unique ID for this coordinator.
    pub id: Uuid,

    /// Coordinator configuration.
    pub config: CoordinatorConfig,

    /// State manager for accessing the database.
    pub state_manager: Arc<dyn DatabaseTrait>,

    /// Collection of ResouceManagers that this coordinator is able to use.
    /// ---
    /// Note that pipelines are only able to use a given resource manager
    /// if it is registered here. (i.e., LocalRun, Docker, CloudRun, etc.)
    resource_managers: Arc<HashMap<Uuid, Arc<dyn ResourceManager>>>,

    /// Contains info about all active launchers,
    /// ---
    /// It is also used as a routing table for gRPC calls
    /// to reach the appropriate launcher.
    ///
    /// The map is `<launcher_id, ManagedLauncher>`
    active_launchers: Arc<RwLock<HashMap<Uuid, ManagedLauncher>>>,

    /// Used internally for polling completion/panic
    /// of RunLauncher tokio tasks.
    laucher_lifecycles: FuturesUnordered<MonitoredLauncherTask>,

    /// Central status Receiver for all RunLauncher tasks.
    /// This is used to send status updates to the coordinator.
    launcher_status_rx: Arc<Mutex<mpsc::Receiver<LauncherStatus>>>,

    /// Status Sender for all RunLauncher tasks.
    /// This is cloned into each new RunLauncher task,
    /// and is where updates should be sent.
    launcher_status_tx_template: mpsc::Sender<LauncherStatus>,

    /// Receiver for commands to the coordinator.
    /// This is used to send requests received from the API,
    /// and send commands to the coordinator to act on those requests.
    api_command_rx: mpsc::Receiver<CoordinatorCommand>,

    /// Receiver for commands from the global supervisor.
    /// This is used to send commands to the coordinator to act on those requests.
    supervisor_command_rx: mpsc::Receiver<SupervisorCommand>,
}

impl Coordinator {
    pub fn new(
        config: CoordinatorConfig,
        state_manager: Arc<dyn DatabaseTrait>,
        resource_managers: Arc<HashMap<Uuid, Arc<dyn ResourceManager>>>,
        api_command_rx: mpsc::Receiver<CoordinatorCommand>,
        supervisor_command_rx: mpsc::Receiver<SupervisorCommand>,
    ) -> Self {
        let (status_tx, status_rx) = mpsc::channel(config.max_launchers * 2);
        Self {
            id: Uuid::new_v4(),
            config,
            state_manager,
            resource_managers,
            active_launchers: Arc::new(RwLock::new(HashMap::new())),
            laucher_lifecycles: FuturesUnordered::new(),
            launcher_status_rx: Arc::new(Mutex::new(status_rx)),
            launcher_status_tx_template: status_tx.clone(),
            api_command_rx,
            supervisor_command_rx,
        }
    }

    #[allow(unused_variables)]
    pub async fn run_main_loop(&mut self) -> Result<(), Error> {
        let pid = process::id() as i32;
        let hostname = hostname::get()
            .unwrap_or_else(|_| "Unknown".to_string().into())
            .into_string()
            .unwrap_or_else(|_| "Unknown".to_string());

        self.state_manager
            .create_new_coordinator(self.id, hostname, pid, "v0.1".to_string(), None)
            .await?;

        // TODO(thegenem0):
        // Let's do some:
        // - recovery logic
        // - health checking
        // - other bootstrapping
        //
        // once that's done, then call this:

        self.state_manager
            .update_coordinator_status(self.id, CoordinatorStatus::Running)
            .await?;

        // Once all of this is verified, and we transitioned to running,
        // we can start the main loop.

        loop {
            tokio::select! {
                Some(command) = self.supervisor_command_rx.recv() => {
                    match command {
                        SupervisorCommand::InitiateShutdown { ack_channel } => {
                            self.handle_coordinator_teardown(Some(ack_channel)).await?;
                            return Ok(());
                        }
                    }
                },
                Some(command) = self.api_command_rx.recv() => {
                    match command {
                        CoordinatorCommand::SubmitPipeline { pipeline_id, run_id_override, response_oneshot } => {
                            println!("Coordinator received SubmitPipeline command for pipeline {}", pipeline_id);

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

                        // Try to gracefully shut down the coordinator.
                        self.handle_coordinator_teardown(None).await?;

                        return Err(Error::Internal("Launcher status channel closed on Coordinator side".to_string()));
                    }
                },

                Some((launcher_id, join_outcome)) = self.laucher_lifecycles.next(), if !self.laucher_lifecycles.is_empty() => {
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

    async fn handle_coordinator_teardown(
        &mut self,
        ack_channel: Option<oneshot::Sender<bool>>,
    ) -> Result<(), Error> {
        self.state_manager
            .update_coordinator_status(self.id, CoordinatorStatus::Terminating)
            .await?;

        // TODO(thegenem0):
        // This is where we should do some cleanup.
        // - Stop all running pipelines
        // - Stop all running launchers
        // - Clean up coordinator resources
        //
        // Once that's done, we can transition to terminated.

        self.state_manager
            .update_coordinator_status(self.id, CoordinatorStatus::Terminated)
            .await?;

        // Acknowledge the supervisor command.
        // This signals that the command was received and processed.
        if let Some(ack_channel) = ack_channel {
            ack_channel
                .send(true)
                .map_err(|_| Error::Internal("Failed to ack supervisor command".to_string()))?;
        }

        Ok(())
    }

    async fn spawn_new_run_launcher(&mut self) -> Result<Uuid, Error> {
        println!("Coordinator spawning new run launcher");
        let launcher_id = Uuid::new_v4();
        let (command_tx, command_rx) = mpsc::channel(10);

        let state_mgr_clone = Arc::clone(&self.state_manager);
        let resource_mgrs_clone = Arc::clone(&self.resource_managers);
        let status_tx_clone = self.launcher_status_tx_template.clone();

        let launcher_task_handle = tokio::spawn(async move {
            let mut launcher = Launcher::new(
                launcher_id,
                LauncherConfig {
                    config: serde_json::Value::default(),
                },
                state_mgr_clone,
                resource_mgrs_clone,
                command_rx,
                status_tx_clone,
            );

            let launcher_run_result = launcher.run_loop().await;

            if let Err(e) = &launcher_run_result {
                eprintln!("Launcher task failed: {}", e);
            }

            launcher_run_result
        });

        // Add the task handle to the lifecycles polling list.
        self.laucher_lifecycles.push(MonitoredLauncherTask {
            launcher_id,
            handle: launcher_task_handle,
        });

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
        launcher_id: Uuid,
        pipeline_id: Uuid,
        _run_id_override: Option<Uuid>,
    ) -> Result<(), Error> {
        let pipeline_def = self
            .state_manager
            .get_pipeline_definition(pipeline_id)
            .await?;

        match pipeline_def {
            Some(_pipeline_def) => {
                println!(
                    "Coordinator assigning pipeline {} to launcher {}",
                    pipeline_id, launcher_id
                );
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
}
