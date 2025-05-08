use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DeriveActiveEnum, EnumIter};

#[derive(Iden)]
#[iden = "task_definitions"]
pub enum TaskDefinition {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "pipeline_id"]
    PipelineId,
    #[iden = "name"]
    Name,
    #[iden = "description"]
    Description,
    #[iden = "config_schema"]
    ConfigSchema,
    #[iden = "user_code_metadata"]
    UserCodeMetadata,
}

#[derive(Iden)]
#[iden = "task_dependencies"]
pub enum TaskDependency {
    Table,
    #[iden = "pipeline_id"]
    PipelineId,
    #[iden = "source_task_definition_id"]
    SourceTaskDefinitionId,
    #[iden = "source_output_name"]
    SourceOutputName,
    #[iden = "target_task_definition_id"]
    TargetTaskDefinitionId,
    #[iden = "target_input_name"]
    TargetInputName,
}

#[derive(Iden)]
#[iden = "task_instances"]
pub enum TaskInstance {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "run_id"]
    RunId,
    #[iden = "definition_id"]
    DefinitionId,
    #[iden = "name"]
    Name,
    #[iden = "status"]
    Status,
    #[iden = "attempts"]
    Attempts,
    #[iden = "start_time"]
    StartTime,
    #[iden = "end_time"]
    EndTime,
    #[iden = "worker_id"]
    WorkerId,
    #[iden = "output_metadata"]
    OutputMetadata,
    #[iden = "error_data"]
    ErrorData,
    #[iden = "last_heartbeat"]
    LastHeartbeat,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "updated_at"]
    UpdatedAt,
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "task_status")]
pub enum DbTaskStatus {
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
