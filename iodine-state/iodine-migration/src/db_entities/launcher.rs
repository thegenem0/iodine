use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query;

#[derive(Iden)]
#[iden = "launchers"]
pub enum Launcher {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "coordinator_id"]
    CoordinatorId,
    #[iden = "assigned_pipeline_def_id"]
    AssignedPipelineDefId,
    #[iden = "started_at"]
    StartedAt,
    #[iden = "terminated_at"]
    TerminatedAt,
}
