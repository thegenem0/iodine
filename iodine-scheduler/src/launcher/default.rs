use std::{collections::HashMap, sync::Arc};

use iodine_common::{
    command::CommandRouter,
    error::Error,
    pipeline::{PipelineDefinition, PipelineRunStatus},
    state::DatabaseTrait,
    task::TaskStatus,
};
use uuid::Uuid;

use crate::resource_manager::default::ResourceManager;

use super::{LauncherCommand, LauncherConfig, PipelineExecutionGraph};

pub struct Launcher {
    pub id: Uuid,
    pub config: LauncherConfig,
    pub command_router: Arc<CommandRouter>,
    pub state_manager: Arc<dyn DatabaseTrait>,
    pub resource_managers: Arc<HashMap<Uuid, Arc<dyn ResourceManager>>>,
    coordinator_id: Uuid,
    current_pipeline_id: Option<Uuid>,
    task_states: HashMap<Uuid, String>,
    execution_graph: Option<PipelineExecutionGraph>,
    active_workers: HashMap<Uuid, (Uuid, u32)>, // Tracks workers -> (task_id, task_attempt)
}

impl Launcher {
    pub fn new(
        id: Uuid,
        coordinator_id: Uuid,
        config: LauncherConfig,
        command_router: Arc<CommandRouter>,
        state_manager: Arc<dyn DatabaseTrait>,
        resource_managers: Arc<HashMap<Uuid, Arc<dyn ResourceManager>>>,
    ) -> Self {
        Self {
            id,
            config,
            command_router,
            state_manager,
            resource_managers,
            coordinator_id,
            current_pipeline_id: None,
            task_states: HashMap::new(),
            execution_graph: None,
            active_workers: HashMap::new(),
        }
    }

    pub async fn run_loop(&mut self) -> Result<(), Error> {
        let mut command_rx = self
            .command_router
            .subscribe::<LauncherCommand>(Some(self.id))
            .await?;

        loop {
            tokio::select! {
                Some(command) = command_rx.recv() => {
                    match command {
                        LauncherCommand::ExecutePipeline { pipeline_definition } => {
                            self.initialize_pipeline(pipeline_definition).await?;
                        }
                        LauncherCommand::Terminate { ack_chan } => {
                            if (self.teardown_launcher().await).is_err() {
                                let _ = ack_chan.send(false);
                            } else {
                                let _ = ack_chan.send(true);
                            }
                        }
                    }
                },

            }
        }
    }

    async fn initialize_pipeline(
        &self,
        pipeline_definition: Arc<PipelineDefinition>,
    ) -> Result<(), Error> {
        println!(
            "Launcher {} initializing pipeline {}",
            self.id, pipeline_definition.info.id
        );

        println!("Doing some stuff");

        println!("Launcher successfully initialized, ready to execute pipeline.");

        Ok(())
    }

    async fn process_execution_cycle(&self) -> Result<Option<PipelineRunStatus>, Error> {
        todo!()
    }

    async fn launch_worker(&self, _task_id: Uuid, _attempt: u32) -> Result<(), Error> {
        todo!()
    }

    async fn handle_worker_outcome(
        &self,
        _worker_id: Uuid,
        _task_id: Uuid,
        _attempt: u32,
        _status: TaskStatus,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn teardown_launcher(&self) -> Result<(), Error> {
        println!("Launcher {} shutting down.", self.id);

        self.state_manager
            .terminate_launcher(self.coordinator_id, self.id)
            .await?;

        Ok(())
    }

    // pub async fn run_loop(&mut self) -> Result<(), String>
    //     // This loop would:
    //     // 1. Wait for an ExecuteDag command.
    //     // 2. Initialize internal state for that DAG.
    //     // 3. Enter an inner loop to:
    //     //    a. Identify ready nodes.
    //     //    b. Launch workers for them.
    //     //    c. Monitor active workers (could involve a separate monitoring task per worker or periodic polling).
    //     //    d. Handle worker completions/failures, update node states.
    //     //    e. Persist state changes via StateManagerClient.
    //     //    f. Repeat until DAG completion or failure.
    //     // 4. Send final DAG status to Coordinator via status_sender.
    //     // 5. Clean up internal state and wait for next command.
}
