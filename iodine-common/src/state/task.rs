use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    error::Error,
    task::{TaskDefinition, TaskRun, TaskRunStatus},
};

use super::base::BaseDbTrait;

#[async_trait]
pub trait TaskDbTrait: BaseDbTrait {
    /// Gets a task definition by `definition_id`
    /// ---
    async fn get_task_definition(&self, task_def_id: Uuid)
    -> Result<Option<TaskDefinition>, Error>;

    /// Lists all task definitions
    /// ---
    /// Can optionally filter by `pipeline_id`
    /// TODO(thegenem0): Should this be paginated?
    async fn list_task_definitions(
        &self,
        pipeline_def_id: Option<Uuid>,
    ) -> Result<(Vec<TaskDefinition>, u64), Error>;

    /// Gets a task instance by `task_id`
    /// ---
    async fn get_task_run(&self, task_run_id: Uuid) -> Result<Option<TaskRun>, Error>;

    /// Lists all task instances
    /// ---
    /// Can optionally filter by `run_id`
    /// TODO(thegenem0): Should this be paginated?
    async fn list_task_runs(
        &self,
        pipeline_run_id: Option<Uuid>,
    ) -> Result<(Vec<TaskRun>, u64), Error>;

    /// Updates the status of a task instance
    /// ---
    /// Reason or other metadata can be provided
    /// for logging purposes.
    /// If `error_data` is provided, it will be
    /// stored along with the status change.
    async fn update_task_run_status(
        &self,
        task_run_id: Uuid,
        new_status: TaskRunStatus,
        metadata: Option<serde_json::Value>,
        error_data: Option<serde_json::Value>,
    ) -> Result<(), Error>;

    /// Creates a new `TaskRun` for a `TaskDefinition` with `run_id`
    /// ---
    /// This is used to track the status of
    /// long-running tasks.
    async fn create_task_run(
        &self,
        run_id: Uuid,
        task_def_id: Uuid,
        attempt: u32,
        status: TaskRunStatus,
        output: Option<serde_json::Value>,
        message: Option<String>,
    ) -> Result<(), Error>;
}
