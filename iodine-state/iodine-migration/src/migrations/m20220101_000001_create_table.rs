use sea_orm_migration::{
    prelude::{extension::postgres::Type, *},
    sea_orm::{ActiveEnum, DbBackend, Schema},
};

use crate::db_entities::{
    DbPipelineRunStatus, DbTaskStatus, EventLog, PipelineDefinition, PipelineRun, TaskDefinition,
    TaskDependency, TaskInstance,
};

const IDX_PIPELINE_RUNS_STATUS: &str = "idx_pipeline_runs_status";
const IDX_PIPELINE_RUNS_DEFINITION_ID: &str = "idx_pipeline_runs_definition_id";
const IDX_PIPELINE_RUNS_CREATED_AT: &str = "idx_pipeline_runs_created_at";
const IDX_TASK_INSTANCES_RUN_ID_STATUS: &str = "idx_task_instances_run_id_status";
const IDX_TASK_INSTANCES_STATUS: &str = "idx_task_instances_status";
const IDX_TASK_INSTANCES_RUN_ID_DEFINITION_ID: &str = "idx_task_instances_run_id_definition_id";
const IDX_TASK_INSTANCES_UPDATED_AT: &str = "idx_task_instances_updated_at";
const IDX_EVENT_LOG_RUN_ID_TIMESTAMP: &str = "idx_event_log_run_id_timestamp";
const IDX_EVENT_LOG_TASK_ID_TIMESTAMP: &str = "idx_event_log_task_id_timestamp";
const IDX_EVENT_LOG_EVENT_TYPE: &str = "idx_event_log_event_type";
const IDX_EVENT_LOG_TIMESTAMP: &str = "idx_event_log_timestamp";
const IDX_TASK_DEF_PIPELINE_ID: &str = "idx_task_def_pipeline_id";

const IDX_NAMES: &[&str] = &[
    IDX_PIPELINE_RUNS_STATUS,
    IDX_PIPELINE_RUNS_DEFINITION_ID,
    IDX_PIPELINE_RUNS_CREATED_AT,
    IDX_TASK_INSTANCES_RUN_ID_STATUS,
    IDX_TASK_INSTANCES_STATUS,
    IDX_TASK_INSTANCES_RUN_ID_DEFINITION_ID,
    IDX_TASK_INSTANCES_UPDATED_AT,
    IDX_EVENT_LOG_RUN_ID_TIMESTAMP,
    IDX_EVENT_LOG_TASK_ID_TIMESTAMP,
    IDX_EVENT_LOG_EVENT_TYPE,
    IDX_EVENT_LOG_TIMESTAMP,
    IDX_TASK_DEF_PIPELINE_ID,
];

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        if db_backend == DbBackend::Postgres {
            let schema = Schema::new(DbBackend::Postgres);

            manager
                .create_type(schema.create_enum_from_active_enum::<DbPipelineRunStatus>())
                .await?;

            manager
                .create_type(schema.create_enum_from_active_enum::<DbTaskStatus>())
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(PipelineDefinition::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PipelineDefinition::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PipelineDefinition::Name).string())
                    .col(ColumnDef::new(PipelineDefinition::Description).text())
                    .col(ColumnDef::new(PipelineDefinition::DefaultBackend).text()) // TODO(thegenem0): maybe enum?
                    .col(ColumnDef::new(PipelineDefinition::DefaultTags).json_binary())
                    .col(ColumnDef::new(PipelineDefinition::Metadata).json_binary())
                    .col(
                        ColumnDef::new(PipelineDefinition::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(PipelineDefinition::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PipelineRun::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PipelineRun::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PipelineRun::DefinitionId).uuid().not_null())
                    .col(
                        ColumnDef::new(PipelineRun::Status)
                            .custom(DbPipelineRunStatus::name())
                            .not_null(),
                    )
                    .col(ColumnDef::new(PipelineRun::Tags).json_binary())
                    .col(ColumnDef::new(PipelineRun::TriggerInfo).json_binary())
                    .col(ColumnDef::new(PipelineRun::StartTime).timestamp_with_time_zone()) // Nullable
                    .col(ColumnDef::new(PipelineRun::EndTime).timestamp_with_time_zone()) // Nullable
                    .col(
                        ColumnDef::new(PipelineRun::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(PipelineRun::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_pipeline_run_def")
                            .from(PipelineRun::Table, PipelineRun::DefinitionId)
                            .to(PipelineDefinition::Table, PipelineDefinition::Id)
                            // TODO(thegenem0):
                            // RESTRICT if preventing deletion of definitions
                            // with history is needed
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TaskDefinition::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskDefinition::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskDefinition::PipelineId).uuid().not_null())
                    .col(ColumnDef::new(TaskDefinition::Name).text().not_null())
                    .col(ColumnDef::new(TaskDefinition::Description).text())
                    .col(ColumnDef::new(TaskDefinition::ConfigSchema).json_binary())
                    .col(ColumnDef::new(TaskDefinition::UserCodeMetadata).json_binary())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_def_pipeline")
                            .from(TaskDefinition::Table, TaskDefinition::PipelineId)
                            .to(PipelineDefinition::Table, PipelineDefinition::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TaskDependency::Table)
                    .if_not_exists()
                    .primary_key(
                        Index::create()
                            .col(TaskDependency::PipelineId)
                            .col(TaskDependency::SourceTaskDefinitionId)
                            .col(TaskDependency::TargetTaskDefinitionId)
                            .col(TaskDependency::TargetInputName),
                    )
                    .col(ColumnDef::new(TaskDependency::PipelineId).uuid().not_null())
                    .col(
                        ColumnDef::new(TaskDependency::SourceTaskDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskDependency::SourceOutputName)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskDependency::TargetTaskDefinitionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskDependency::TargetInputName)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dep_source_task")
                            .from(
                                TaskDependency::Table,
                                TaskDependency::SourceTaskDefinitionId,
                            )
                            .to(TaskDefinition::Table, TaskDefinition::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_dep_target_task")
                            .from(
                                TaskDependency::Table,
                                TaskDependency::TargetTaskDefinitionId,
                            )
                            .to(TaskDefinition::Table, TaskDefinition::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TaskInstance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskInstance::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskInstance::RunId).uuid().not_null())
                    .col(ColumnDef::new(TaskInstance::DefinitionId).uuid().not_null())
                    .col(ColumnDef::new(TaskInstance::Name).text().not_null())
                    .col(
                        ColumnDef::new(TaskInstance::Status)
                            .custom(DbTaskStatus::name())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskInstance::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(TaskInstance::WorkerId).uuid())
                    .col(ColumnDef::new(TaskInstance::OutputMetadata).json_binary())
                    .col(ColumnDef::new(TaskInstance::ErrorData).json_binary())
                    .col(ColumnDef::new(TaskInstance::StartTime).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskInstance::EndTime).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskInstance::LastHeartbeat).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(TaskInstance::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskInstance::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_instance_run")
                            .from(TaskInstance::Table, TaskInstance::RunId)
                            .to(PipelineRun::Table, PipelineRun::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EventLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventLog::EventId) // BIGSERIAL for auto-incrementing PK
                            .big_integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EventLog::Timestamp)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(EventLog::RunId).uuid())
                    .col(ColumnDef::new(EventLog::TaskId).uuid())
                    .col(ColumnDef::new(EventLog::EventType).text().not_null())
                    .col(ColumnDef::new(EventLog::Message).text())
                    .col(ColumnDef::new(EventLog::Metadata).json_binary())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_event_run")
                            .from(EventLog::Table, EventLog::RunId)
                            .to(PipelineRun::Table, PipelineRun::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_PIPELINE_RUNS_STATUS)
                    .table(PipelineRun::Table)
                    .col(PipelineRun::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_PIPELINE_RUNS_DEFINITION_ID)
                    .table(PipelineRun::Table)
                    .col(PipelineRun::DefinitionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_PIPELINE_RUNS_CREATED_AT)
                    .table(PipelineRun::Table)
                    .col(PipelineRun::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_DEF_PIPELINE_ID)
                    .table(TaskDefinition::Table)
                    .col(TaskDefinition::PipelineId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_RUN_ID_STATUS)
                    .table(TaskInstance::Table)
                    .col(TaskInstance::RunId)
                    .col(TaskInstance::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_STATUS)
                    .table(TaskInstance::Table)
                    .col(TaskInstance::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_RUN_ID_DEFINITION_ID)
                    .table(TaskInstance::Table)
                    .col(TaskInstance::RunId)
                    .col(TaskInstance::DefinitionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_UPDATED_AT)
                    .table(TaskInstance::Table)
                    .col(TaskInstance::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_EVENT_LOG_RUN_ID_TIMESTAMP)
                    .table(EventLog::Table)
                    .col(EventLog::RunId)
                    .col(EventLog::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_EVENT_LOG_TASK_ID_TIMESTAMP)
                    .table(EventLog::Table)
                    .col(EventLog::TaskId)
                    .col(EventLog::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_EVENT_LOG_EVENT_TYPE)
                    .table(EventLog::Table)
                    .col(EventLog::EventType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_EVENT_LOG_TIMESTAMP)
                    .table(EventLog::Table)
                    .col(EventLog::Timestamp)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();

        for idx_name in IDX_NAMES {
            manager
                .drop_index(Index::drop().name(*idx_name).to_owned())
                .await?;
        }

        manager
            .drop_table(Table::drop().table(EventLog::Table).if_exists().to_owned())
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(TaskInstance::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(PipelineRun::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(TaskDependency::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(TaskDefinition::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(PipelineDefinition::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        if db_backend == DbBackend::Postgres {
            manager
                .drop_type(Type::drop().name(DbPipelineRunStatus::name()).to_owned())
                .await?;

            manager
                .drop_type(Type::drop().name(DbTaskStatus::name()).to_owned())
                .await?;
        }

        Ok(())
    }
}
