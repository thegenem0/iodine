use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    error::{Error, WorkerError},
    pipeline::{PipelineDefinition, PipelineInfo, PipelineRun, PipelineRunStatus},
};

use super::base::BaseDbTrait;

#[async_trait]
pub trait PipelineDbTrait: BaseDbTrait {
    //// --- READs --- ////

    /// Gets the metadata for a specific pipeline definition by `pipeline_id`.
    /// ---
    /// This includes all stored information about the pipeline definition.
    /// NOTE(_): This gets the full definition
    /// and can be expensive for large pipelines
    async fn get_pipeline_definition(
        &self,
        pipeline_id: Uuid,
    ) -> Result<Option<PipelineDefinition>, Error>;

    /// Lists all pipeline definitions
    /// ---
    /// TODO(thegenem0): Should this be paginated?
    async fn list_pipeline_definitions(&self) -> Result<(Vec<PipelineInfo>, u64), Error>;

    /// Gets the metadata for a specific pipeline run by `run_id`.
    /// ---
    async fn get_pipeline_run(&self, run_id: Uuid) -> Result<Option<PipelineRun>, Error>;

    /// Gets all active runs (currently `Running` or `Queued`)
    /// ---
    /// Returns a list of `(run_id, run_status)` tuples
    async fn get_active_runs(&self) -> Result<HashMap<Uuid, PipelineRunStatus>, Error>;

    /// Lists all pipeline runs
    /// ---
    /// TODO(thegenem0): Should this be paginated?
    async fn list_runs(&self) -> Result<(Vec<PipelineRun>, u64), Error>;

    /// Registers a new pipeline from a `CodeRegistry`
    /// ---
    /// Only stores the information in the DB
    /// It `DOES NOT!` run or schedule the pipeline
    async fn register_pipeline(&self, definition: &PipelineDefinition) -> Result<(), Error>;

    /// Deregisters a pipeline by `pipeline_id`
    /// ---
    /// Only removes the information from the DB
    /// Should only be called in an import loop
    /// acting as a cleanup call resulting from a
    /// `CodeRegistry` change where the pipeline
    /// is no longer available.
    async fn deregister_pipeline(&self, pipeline_id: Uuid) -> Result<(), Error>;

    /// Creates a new pipeline run
    /// ---
    /// It creates a new run record,
    /// the corresponding task records,
    /// and sets initial statuses.
    /// It does `NOT` schedule the run.
    async fn create_pipeline_run(
        &self,
        pipeline_run_id: Uuid,
        pipeline_def_id: Uuid,
        launcher_id: Uuid,
        initial_run_status: PipelineRunStatus,
    ) -> Result<(), Error>;

    /// Updates a pipeline run's status
    async fn update_pipeline_run_status(
        &self,
        run_id: Uuid,
        new_status: PipelineRunStatus,
        message: Option<String>,
    ) -> Result<(), Error>;

    /// Finalizes a pipeline run
    /// ---
    /// It updates the run record,
    /// and sets fields to indicate completion.
    /// This should be called on both success and
    /// failure to signal termination to the coordinator.
    async fn finalize_run(
        &self,
        run_id: Uuid,
        final_status: PipelineRunStatus,
        error_info: Option<&WorkerError>,
    ) -> Result<(), Error>;
}
