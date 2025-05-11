use std::{collections::HashMap, sync::Arc};

use iodine_common::{
    error::Error,
    pipeline::{PipelineDefinition, PipelineRunStatus},
    state::DatabaseTrait,
    task::TaskStatus,
};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::resource_manager::default::ResourceManager;

use super::{LauncherCommand, LauncherConfig, LauncherStatus, PipelineExecutionGraph};

pub struct Launcher {
    pub id: Uuid,
    pub config: LauncherConfig,
    pub state_manager: Arc<dyn DatabaseTrait>,
    pub resource_managers: Arc<HashMap<Uuid, Arc<dyn ResourceManager>>>,
    command_rx: mpsc::Receiver<LauncherCommand>,
    status_tx: mpsc::Sender<LauncherStatus>,
    current_pipeline_id: Option<Uuid>,
    task_states: HashMap<Uuid, String>,
    execution_graph: Option<PipelineExecutionGraph>,
    active_workers: HashMap<Uuid, (Uuid, u32)>, // Tracks workers -> (task_id, task_attempt)
}

impl Launcher {
    pub fn new(
        id: Uuid,
        config: LauncherConfig,
        state_manager: Arc<dyn DatabaseTrait>,
        resource_managers: Arc<HashMap<Uuid, Arc<dyn ResourceManager>>>,
        command_rx: mpsc::Receiver<LauncherCommand>,
        status_tx: mpsc::Sender<LauncherStatus>,
    ) -> Self {
        Self {
            id,
            config,
            state_manager,
            resource_managers,
            command_rx,
            status_tx,
            current_pipeline_id: None,
            task_states: HashMap::new(),
            execution_graph: None,
            active_workers: HashMap::new(),
        }
    }

    pub async fn run_loop(&mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        LauncherCommand::ExecutePipeline { pipeline_definition } => {
                            self.initialize_pipeline(pipeline_definition).await?;
                        }
                        LauncherCommand::Terminate => unimplemented!(),
                    }
                },

            }
        }
    }

    async fn initialize_pipeline(
        &self,
        _pipeline_definition: Arc<PipelineDefinition>,
    ) -> Result<(), Error> {
        todo!()
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

    // async fn initialize_for_dag(&mut self, dag_run_id: DagRunId, dag_def: Arc<DagDefinition>) -> Result<(), String>
    // async fn process_dag_execution_cycle(&mut self) -> Result<Option<DagRunStatus>, String> // Returns final status if DAG ended
    // async fn launch_and_monitor_worker(&self, node_id: NodeId, attempt: u32) -> Result<(), String>
    // async fn handle_worker_outcome(&mut self, worker_id: WorkerInstanceId, node_id: NodeId, attempt: u32, status: WorkerInstanceStatus) -> Result<(), String>
}
