use sea_orm_migration::{
    prelude::*,
    sea_orm::{DeriveActiveEnum, EnumIter},
};

#[derive(Iden)]
#[iden = "pipeline_definitions"]
pub enum PipelineDefinition {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "name"]
    Name,
    #[iden = "description"]
    Description,
    #[iden = "default_backend"]
    DefaultBackend,
    #[iden = "default_tags"]
    DefaultTags,
    #[iden = "metadata"]
    Metadata,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "updated_at"]
    UpdatedAt,
}

#[derive(Iden)]
#[iden = "pipeline_runs"]
pub enum PipelineRun {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "pipeline_def_id"]
    PipelineDefId,
    #[iden = "launcher_id"]
    LauncherId,
    #[iden = "status"]
    Status,
    #[iden = "start_time"]
    StartTime,
    #[iden = "end_time"]
    EndTime,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "updated_at"]
    UpdatedAt,
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "pipeline_run_status"
)]
pub enum DbPipelineRunStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "QUEUED")]
    Queued,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "SUCCEEDED")]
    Succeeded,
    #[sea_orm(string_value = "FAILED")]
    Failed,
    #[sea_orm(string_value = "CANCELLING")]
    Cancelling,
    #[sea_orm(string_value = "CANCELLED")]
    Cancelled,
}
