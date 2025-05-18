use std::{collections::HashMap, sync::Arc};

use iodine_common::{
    error::Error,
    pipeline::{PipelineDefinition, PipelineRunStatus},
    task::{TaskDefinition, TaskRunStatus},
};
use tracing::info;
use uuid::Uuid;

use crate::launcher::execution_graph::PipelineExecutionGraph;

use super::default::Launcher;

impl Launcher {
    pub(super) async fn initialize_pipeline_state(
        &mut self,
        pipeline_definition_arc: Arc<PipelineDefinition>,
    ) -> Result<(), Error> {
        self.cleanup_current_pipeline_state();

        let pipeline_def = &*pipeline_definition_arc;
        let pipeline_def_id = pipeline_def.info.id;
        let pipeline_run_id = Uuid::new_v4();

        self.current_pipeline_definition_id = Some(pipeline_def_id);
        self.current_pipeline_run_id = Some(pipeline_run_id);

        info!(
            "Launcher [{}]: Initializing run_id {} for pipeline_def {}",
            self.id, pipeline_run_id, pipeline_def_id
        );

        let _tasks_map: HashMap<Uuid, TaskDefinition> = pipeline_def
            .task_definitions
            .iter()
            .map(|task| (task.id, task.clone()))
            .collect();

        self.state_manager
            .create_pipeline_run(
                pipeline_run_id,
                pipeline_def.info.id,
                self.id,
                PipelineRunStatus::Pending,
            )
            .await?;

        let graph = PipelineExecutionGraph::new(pipeline_def)?;
        self.execution_graph = Some(graph);

        if let Some(current_graph) = &self.execution_graph {
            for task_def_id in current_graph.get_all_task_ids() {
                let task_def = current_graph
                    .get_task_definition(task_def_id)
                    .ok_or_else(|| {
                        Error::Internal(format!(
                            "Failed to find TaskDefinition for task ID {}",
                            task_def_id
                        ))
                    })?;

                self.task_states.insert(task_def_id, TaskRunStatus::Pending);

                self.state_manager
                    .create_task_run(
                        pipeline_run_id,
                        task_def.id,
                        0,
                        TaskRunStatus::Pending,
                        None,
                        None,
                    )
                    .await?;
            }
        }

        self.state_manager
            .update_pipeline_run_status(pipeline_run_id, PipelineRunStatus::Running, None)
            .await?;

        Ok(())
    }

    pub(super) fn cleanup_current_pipeline_state(&mut self) {
        info!(
            "Launcher [{}]: Cleaning up state for previous/current pipeline.",
            self.id
        );

        self.current_pipeline_definition_id = None;
        self.current_pipeline_run_id = None;
        self.execution_graph = None;
        self.task_states.clear();
        self.task_attempts.clear();
        // Active workers and monitoring tasks are handled by teardown or normal completion.
    }
}
