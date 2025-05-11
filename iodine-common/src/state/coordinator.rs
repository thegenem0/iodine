use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    coordinator::{Coordinator, CoordinatorStatus},
    error::Error,
};

use super::BaseDbTrait;

#[async_trait]
pub trait CoordinatorDbTrait: BaseDbTrait {
    /// Gets coordinator information
    async fn get_coordinator_info(&self) -> Result<Option<Coordinator>, Error>;

    async fn create_new_coordinator(
        &self,
        id: Uuid,
        hostname: String,
        host_pid: i32,
        version: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), Error>;

    async fn update_coordinator_status(
        &self,
        id: Uuid,
        status: CoordinatorStatus,
    ) -> Result<(), Error>;

    async fn register_coordinator_heartbeat(&self, id: Uuid) -> Result<(), Error>;

    async fn assign_pipeline_to_launcher(
        &self,
        launcher_id: Uuid,
        pipeline_id: Uuid,
        run_id_override: Option<Uuid>,
    ) -> Result<(), Error>;
}
