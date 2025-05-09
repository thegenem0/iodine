use async_trait::async_trait;
use iodine_common::{
    error::Error,
    state::TaskDbTrait,
    task::{TaskDefinition, TaskInstance, TaskStatus},
};
use uuid::Uuid;

use crate::db::PostgresStateDb;

#[async_trait]
#[allow(unused_variables)]
impl TaskDbTrait for PostgresStateDb {
    async fn get_task_definition(
        &self,
        definition_id: Uuid,
    ) -> Result<Option<TaskDefinition>, Error> {
        todo!()
    }

    async fn list_task_definitions(
        &self,
        pipeline_id: Option<Uuid>,
    ) -> Result<(Vec<TaskDefinition>, u64), Error> {
        todo!()
    }

    async fn get_task_instance(&self, task_id: Uuid) -> Result<Option<TaskInstance>, Error> {
        todo!()
    }

    async fn list_task_instances(
        &self,
        run_id: Option<Uuid>,
    ) -> Result<(Vec<TaskInstance>, u64), Error> {
        todo!()
    }

    async fn get_prerequisite_statuses(
        &self,
        run_id: Uuid,
        task_id: Uuid,
    ) -> Result<Vec<(Uuid, TaskStatus)>, Error> {
        todo!()
    }

    async fn update_task_status(
        &self,
        task_id: Uuid,
        new_status: TaskStatus,
        metadata: Option<serde_json::Value>,
        error_data: Option<serde_json::Value>,
    ) -> Result<(), Error> {
        todo!()
    }

    async fn enqueue_task(&self, task_id: Uuid) -> Result<(), Error> {
        todo!()
    }

    async fn record_task_heartbeat(&self, task_id: Uuid) -> Result<(), Error> {
        todo!()
    }

    async fn claim_task(&self, task_id: Uuid, worker_id: Uuid) -> Result<TaskInstance, Error> {
        todo!()
    }
}
