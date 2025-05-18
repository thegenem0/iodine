use sea_orm_migration::{
    prelude::{extension::postgres::Type, *},
    sea_orm::{ActiveEnum, DbBackend, Schema},
};

use crate::db_entities::{
    Coordinator, DbCoordinatorStatus, DbPipelineRunStatus, DbTaskRunStatus, EventLog, Launcher,
    PipelineDefinition, PipelineRun, TaskDefinition, TaskRun,
};

const IDX_COORDINATORS_IS_LEADER: &str = "idx_coordinators_is_leader";
const IDX_COORDINATORS_STATUS: &str = "idx_coordinators_status";
const IDX_COORDINATORS_LAST_HEARTBEAT: &str = "idx_coordinators_last_heartbeat";
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
    IDX_COORDINATORS_IS_LEADER,
    IDX_COORDINATORS_STATUS,
    IDX_COORDINATORS_LAST_HEARTBEAT,
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
                .create_type(schema.create_enum_from_active_enum::<DbCoordinatorStatus>())
                .await?;

            manager
                .create_type(schema.create_enum_from_active_enum::<DbPipelineRunStatus>())
                .await?;

            manager
                .create_type(schema.create_enum_from_active_enum::<DbTaskRunStatus>())
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(Coordinator::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Coordinator::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Coordinator::Hostname).text().not_null())
                    .col(ColumnDef::new(Coordinator::HostPid).integer().not_null())
                    .col(
                        ColumnDef::new(Coordinator::IsLeader)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Coordinator::Status)
                            .custom(DbCoordinatorStatus::name())
                            .not_null(),
                    )
                    .col(ColumnDef::new(Coordinator::Version).text().not_null())
                    .col(
                        ColumnDef::new(Coordinator::LastHeartbeat)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Coordinator::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Coordinator::TerminatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Coordinator::Metadata).json_binary())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Launcher::Table)
                    .if_not_exists()
                    .primary_key(
                        Index::create()
                            .col(Launcher::Id)
                            .col(Launcher::CoordinatorId),
                    )
                    .col(ColumnDef::new(Launcher::Id).uuid().not_null())
                    .col(ColumnDef::new(Launcher::CoordinatorId).uuid().not_null())
                    .col(ColumnDef::new(Launcher::AssignedPipelineDefId).uuid())
                    .col(ColumnDef::new(Launcher::StartedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Launcher::TerminatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

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
                    .col(ColumnDef::new(PipelineRun::PipelineDefId).uuid().not_null())
                    .col(ColumnDef::new(PipelineRun::LauncherId).uuid().not_null())
                    .col(
                        ColumnDef::new(PipelineRun::Status)
                            .custom(DbPipelineRunStatus::name())
                            .not_null(),
                    )
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
                            .from(PipelineRun::Table, PipelineRun::PipelineDefId)
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
                    .col(
                        ColumnDef::new(TaskDefinition::PipelineDefId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TaskDefinition::Name).text().not_null())
                    .col(ColumnDef::new(TaskDefinition::Description).text())
                    .col(
                        ColumnDef::new(TaskDefinition::ExecutionContext)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TaskDefinition::MaxAttempts).integer())
                    .col(ColumnDef::new(TaskDefinition::DependsOn).array(ColumnType::Uuid))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_def_pipeline")
                            .from(TaskDefinition::Table, TaskDefinition::PipelineDefId)
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
                    .table(TaskRun::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TaskRun::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(TaskRun::PipelineRunId).uuid().not_null())
                    .col(ColumnDef::new(TaskRun::TaskDefId).uuid().not_null())
                    .col(
                        ColumnDef::new(TaskRun::Status)
                            .custom(DbTaskRunStatus::name())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TaskRun::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(TaskRun::OutputMetadata).json_binary())
                    .col(ColumnDef::new(TaskRun::Message).text())
                    .col(ColumnDef::new(TaskRun::StartTime).timestamp_with_time_zone())
                    .col(ColumnDef::new(TaskRun::EndTime).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(TaskRun::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TaskRun::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_task_instance_run")
                            .from(TaskRun::Table, TaskRun::PipelineRunId)
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
                    .col(ColumnDef::new(EventLog::PipelineRunId).uuid())
                    .col(ColumnDef::new(EventLog::TaskRunId).uuid())
                    .col(ColumnDef::new(EventLog::EventType).text().not_null())
                    .col(ColumnDef::new(EventLog::Message).text())
                    .col(ColumnDef::new(EventLog::Metadata).json_binary())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_COORDINATORS_IS_LEADER)
                    .table(Coordinator::Table)
                    .col(Coordinator::IsLeader)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_COORDINATORS_STATUS)
                    .table(Coordinator::Table)
                    .col(Coordinator::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_COORDINATORS_LAST_HEARTBEAT)
                    .table(Coordinator::Table)
                    .col(Coordinator::LastHeartbeat)
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
                    .col(PipelineRun::PipelineDefId)
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
                    .col(TaskDefinition::PipelineDefId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_RUN_ID_STATUS)
                    .table(TaskRun::Table)
                    .col(TaskRun::PipelineRunId)
                    .col(TaskRun::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_STATUS)
                    .table(TaskRun::Table)
                    .col(TaskRun::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_RUN_ID_DEFINITION_ID)
                    .table(TaskRun::Table)
                    .col(TaskRun::PipelineRunId)
                    .col(TaskRun::TaskDefId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_TASK_INSTANCES_UPDATED_AT)
                    .table(TaskRun::Table)
                    .col(TaskRun::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_EVENT_LOG_RUN_ID_TIMESTAMP)
                    .table(EventLog::Table)
                    .col(EventLog::PipelineRunId)
                    .col(EventLog::Timestamp)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(IDX_EVENT_LOG_TASK_ID_TIMESTAMP)
                    .table(EventLog::Table)
                    .col(EventLog::TaskRunId)
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
            .drop_table(Table::drop().table(TaskRun::Table).if_exists().to_owned())
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
                    .table(TaskDefinition::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Launcher::Table).if_exists().to_owned())
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(Coordinator::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        if db_backend == DbBackend::Postgres {
            manager
                .drop_type(Type::drop().name(DbPipelineRunStatus::name()).to_owned())
                .await?;

            manager
                .drop_type(Type::drop().name(DbTaskRunStatus::name()).to_owned())
                .await?;

            manager
                .drop_type(Type::drop().name(DbCoordinatorStatus::name()).to_owned())
                .await?;
        }

        Ok(())
    }
}
