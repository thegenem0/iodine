use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    error::Error,
    task::{TaskDefinition, TaskInstance, TaskStatus},
};

use super::base::BaseDbTrait;

#[async_trait]
pub trait TaskDbTrait: BaseDbTrait {
    /// Gets a task definition by `definition_id`
    /// ---
    async fn get_task_definition(
        &self,
        definition_id: Uuid,
    ) -> Result<Option<TaskDefinition>, Error>;

    /// Lists all task definitions
    /// ---
    /// Can optionally filter by `pipeline_id`
    /// TODO(thegenem0): Should this be paginated?
    async fn list_task_definitions(
        &self,
        pipeline_id: Option<Uuid>,
    ) -> Result<(Vec<TaskDefinition>, u64), Error>;

    /// Gets a task instance by `task_id`
    /// ---
    async fn get_task_instance(&self, task_id: Uuid) -> Result<Option<TaskInstance>, Error>;

    /// Lists all task instances
    /// ---
    /// Can optionally filter by `run_id`
    /// TODO(thegenem0): Should this be paginated?
    async fn list_task_instances(
        &self,
        run_id: Option<Uuid>,
    ) -> Result<(Vec<TaskInstance>, u64), Error>;

    /// Gets `(task_id, task_status)` tuples for all nodes
    /// that are prerequisites of the node with `task_id`
    /// within a given run (`run_id`).
    async fn get_prerequisite_statuses(
        &self,
        run_id: Uuid,
        task_id: Uuid,
    ) -> Result<HashMap<Uuid, TaskStatus>, Error>;

    /// Updates the status of a task instance
    /// ---
    /// Reason or other metadata can be provided
    /// for logging purposes.
    /// If `error_data` is provided, it will be
    /// stored along with the status change.
    async fn update_task_status(
        &self,
        task_id: Uuid,
        new_status: TaskStatus,
        metadata: Option<serde_json::Value>,
        error_data: Option<serde_json::Value>,
    ) -> Result<(), Error>;

    /// Convenience method enqueue a task for execution
    /// ---
    /// Should be called by an `<impl RunLauncher>` instance
    async fn enqueue_task(&self, task_id: Uuid) -> Result<(), Error>;

    /// Records a task instance's heartbeat
    /// ---
    /// This is used to track the status of
    /// long-running tasks.
    /// TODO(thegenem0): Should accept an `Option<HeartbeatInfo>`
    async fn record_task_heartbeat(&self, task_id: Uuid) -> Result<(), Error>;

    /// Claims a `Queued` task `task_id` for a worker `worker_id`
    ///---
    ///Return `TaskInstance` if successful
    async fn claim_task(&self, task_id: Uuid, worker_id: Uuid) -> Result<TaskInstance, Error>;
}
