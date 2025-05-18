use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DeriveActiveEnum, EnumIter};

#[derive(Iden)]
#[iden = "task_definitions"]
pub enum TaskDefinition {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "pipeline_def_id"]
    PipelineDefId,
    #[iden = "name"]
    Name,
    #[iden = "description"]
    Description,
    #[iden = "execution_context"]
    ExecutionContext,
    #[iden = "max_attempts"]
    MaxAttempts,
    #[iden = "depends_on"]
    DependsOn,
}

#[derive(Iden)]
#[iden = "task_runs"]
pub enum TaskRun {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "pipeline_run_id"]
    PipelineRunId,
    #[iden = "task_def_id"]
    TaskDefId,
    #[iden = "status"]
    Status,
    #[iden = "attempts"]
    Attempts,
    #[iden = "start_time"]
    StartTime,
    #[iden = "end_time"]
    EndTime,
    #[iden = "output_metadata"]
    OutputMetadata,
    #[iden = "message"]
    Message,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "updated_at"]
    UpdatedAt,
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "task_run_status")]
pub enum DbTaskRunStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "QUEUED")]
    Queued,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "RETRYING")]
    Retrying,
    #[sea_orm(string_value = "SUCCEEDED")]
    Succeeded,
    #[sea_orm(string_value = "FAILED")]
    Failed,
    #[sea_orm(string_value = "CANCELLING")]
    Cancelling,
    #[sea_orm(string_value = "CANCELLED")]
    Cancelled,
    #[sea_orm(string_value = "SKIPPED")]
    Skipped,
}
