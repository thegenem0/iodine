use std::{collections::HashMap, process, sync::Arc};

use futures::{StreamExt, stream::FuturesUnordered};
use iodine_common::{
    command::{CommandRouter, DiscoveryCommand},
    coordinator::{CoordinatorCommand, CoordinatorStatus},
    error::Error,
    state::DatabaseTrait,
};
use tokio::sync::{RwLock, oneshot};
use tracing::info;
use uuid::Uuid;

use crate::launcher::{
    LauncherCommand, LauncherConfig, LauncherStatus, default::Launcher,
    exec_mgr_registry::ExecutionManagerRegistry,
};

use super::{CoordinatorConfig, ManagedLauncher, MonitoredLauncherTask, SupervisorCommand};

pub struct Coordinator {
    /// Unique ID for this coordinator.
    pub id: Uuid,

    /// Coordinator configuration.
    pub config: CoordinatorConfig,

    /// Command router for dispatching commands to handlers.
    pub command_router: Arc<CommandRouter>,

    /// State manager for accessing the database.
    pub state_manager: Arc<dyn DatabaseTrait>,

    /// Collection of ResouceManagers that this coordinator is able to use.
    exec_managers: Arc<ExecutionManagerRegistry>,

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
}

impl Coordinator {
    pub async fn new(
        config: CoordinatorConfig,
        command_router: Arc<CommandRouter>,
        state_manager: Arc<dyn DatabaseTrait>,
        exec_managers: Arc<ExecutionManagerRegistry>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            config,
            command_router,
            state_manager,
            exec_managers,
            active_launchers: Arc::new(RwLock::new(HashMap::new())),
            laucher_lifecycles: FuturesUnordered::new(),
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

        // grab receivers for handlers needed by this coordinator
        let mut api_command_rx = self
            .command_router
            .subscribe::<CoordinatorCommand>(None)
            .await?;

        let mut supervisor_command_rx = self
            .command_router
            .subscribe::<SupervisorCommand>(None)
            .await?;

        let mut launcher_status_rx = self
            .command_router
            .subscribe::<LauncherStatus>(None)
            .await?;

        self.command_router
            .dispatch(DiscoveryCommand::InitRegistries, None)
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
                Some(command) = supervisor_command_rx.recv() => {
                    match command {
                        SupervisorCommand::Terminate { ack_chan }=> {
                            if (self.handle_coordinator_teardown().await).is_err() {
                                let _ = ack_chan.send(false);
                            } else {
                                let _ = ack_chan.send(true);
                            }
                            return Ok(());
                        }
                    }
                },
                Some(command) = api_command_rx.recv() => {
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
                        CoordinatorCommand::Terminate { ack_chan } => {
                                if (self.handle_coordinator_teardown().await).is_err() {
                                    let _ = ack_chan.send(false);
                                } else {
                                    let _ = ack_chan.send(true);
                                }

                            },
                            _ => todo!("Handle coordinator command"),
                    }
                },
                Some(status) = launcher_status_rx.recv() => {
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

    async fn handle_coordinator_teardown(&mut self) -> Result<(), Error> {
        self.state_manager
            .update_coordinator_status(self.id, CoordinatorStatus::Terminating)
            .await?;

        let mut errored_launchers = Vec::new();

        for (launcher_id, _) in self.active_launchers.read().await.iter() {
            let (ack_tx, ack_rx) = oneshot::channel::<bool>();
            self.command_router
                .dispatch(
                    LauncherCommand::Terminate { ack_chan: ack_tx },
                    Some(*launcher_id),
                )
                .await?;

            if ack_rx.await.is_err() {
                errored_launchers.push(*launcher_id);
            }
        }

        if !errored_launchers.is_empty() {
            return Err(Error::Internal(format!(
                "Failed to terminate {} launchers",
                errored_launchers.len()
            )));
        }

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

        Ok(())
    }

    async fn spawn_new_run_launcher(&mut self) -> Result<Uuid, Error> {
        println!("Coordinator spawning new run launcher");
        let launcher_id = Uuid::new_v4();
        let coordinator_id = self.id;

        let command_router_clone = Arc::clone(&self.command_router);
        let state_mgr_clone = Arc::clone(&self.state_manager);
        let resource_mgrs_clone = Arc::clone(&self.exec_managers);

        self.command_router
            .register_instanced_handler::<LauncherCommand>(launcher_id)
            .await?;

        let mut launcher = Launcher::new(
            launcher_id,
            coordinator_id,
            LauncherConfig {
                config: serde_json::Value::default(),
                scheduler_tick_interval: None,
                worker_polling_interval: None,
                worker_channel_buffer_size: None,
            },
            command_router_clone,
            state_mgr_clone,
            resource_mgrs_clone,
        )
        .await?;

        let launcher_task_handle = tokio::spawn(async move {
            info!("Starting launcher task for launcher {}", launcher.id);
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
            current_pipeline_id: None, // We don't have any pipelines assigned yet
            current_run_id: None,
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
        let pipeline_def_opt = self
            .state_manager
            .get_pipeline_definition(pipeline_id)
            .await?;

        let pipeline_def = match pipeline_def_opt {
            Some(pipeline_def) => pipeline_def,
            None => {
                return Err(Error::NotFound {
                    resource_type: "PipelineDefinition".to_string(),
                    resource_id: pipeline_id.to_string(),
                });
            }
        };

        self.state_manager
            .assign_pipeline_to_launcher(self.id, launcher_id, pipeline_id, None)
            .await?;

        {
            let mut active_launchers_wlock = self.active_launchers.write().await;
            if let Some(mut_launcher) = active_launchers_wlock.get_mut(&launcher_id) {
                mut_launcher.current_pipeline_id = Some(pipeline_id);
            } else {
                eprintln!(
                    "CRITICAL: Failed to find launcher {} in active_launchers map during assignment.",
                    launcher_id
                );
                return Err(Error::NotFound {
                    resource_type: "ManagedLauncher".to_string(),
                    resource_id: launcher_id.to_string(),
                });
            }
        } // Release write lock

        let exec_cmd = LauncherCommand::ExecutePipeline {
            pipeline_definition: Arc::new(pipeline_def),
        };

        self.command_router
            .dispatch(exec_cmd, Some(launcher_id))
            .await?;

        Ok(())
    }

    async fn recover_state_on_startup(&self) -> Result<(), Error> {
        todo!()
    }
}
