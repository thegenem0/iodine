use crate::entities::{
    coordinators, event_log, pipeline_definitions, pipeline_runs, sea_orm_active_enums,
    task_definitions, task_instances,
};
use iodine_common::{
    coordinator::{Coordinator, CoordinatorStatus},
    error::Error,
    event::{EventLogRecord, EventType},
    pipeline::{PipelineBackend, PipelineDefinition, PipelineInfo, PipelineRun, PipelineRunStatus},
    task::{TaskDefinition, TaskInstance, TaskStatus},
};
use sea_orm::{
    DbErr,
    prelude::Expr,
    sea_query::{Alias, SimpleExpr},
};

const COORDINATOR_STATUS_DB_ENUM_NAME: &str = "coordinator_status";
const PIPELINE_RUN_STATUS_DB_ENUM_NAME: &str = "pipeline_run_status";
const TASK_STATUS_DB_ENUM_NAME: &str = "task_status";

pub(crate) fn db_error_to_domain(e: DbErr) -> Error {
    Error::Database(e.to_string())
}

pub(crate) fn coordinator_to_domain(model: coordinators::Model) -> Coordinator {
    Coordinator {
        id: model.id,
        hostname: model.hostname,
        host_pid: model.host_pid,
        is_leader: model.is_leader,
        status: coordinator_status_to_domain(model.status),
        version: model.version,
        last_heartbeat: model.last_heartbeat.into(),
        started_at: model.started_at.into(),
        terminated_at: model.terminated_at.map(|t| t.into()),
        metadata: model.metadata,
    }
}

pub(crate) fn coordinator_status_to_domain(
    model: sea_orm_active_enums::CoordinatorStatus,
) -> CoordinatorStatus {
    match model {
        sea_orm_active_enums::CoordinatorStatus::Pending => CoordinatorStatus::Pending,
        sea_orm_active_enums::CoordinatorStatus::Running => CoordinatorStatus::Running,
        sea_orm_active_enums::CoordinatorStatus::Terminating => CoordinatorStatus::Terminating,
        sea_orm_active_enums::CoordinatorStatus::Terminated => CoordinatorStatus::Terminated,
    }
}

pub(crate) fn domain_coordinator_status_to_db(
    model: CoordinatorStatus,
) -> sea_orm_active_enums::CoordinatorStatus {
    match model {
        CoordinatorStatus::Pending => sea_orm_active_enums::CoordinatorStatus::Pending,
        CoordinatorStatus::Running => sea_orm_active_enums::CoordinatorStatus::Running,
        CoordinatorStatus::Terminating => sea_orm_active_enums::CoordinatorStatus::Terminating,
        CoordinatorStatus::Terminated => sea_orm_active_enums::CoordinatorStatus::Terminated,
    }
}

pub(crate) fn coordinator_status_as_expr(model: CoordinatorStatus) -> SimpleExpr {
    Expr::val(model.to_string()).cast_as(Alias::new(COORDINATOR_STATUS_DB_ENUM_NAME))
}

pub(crate) fn pipeline_definition_to_domain(
    pipeline_definition: pipeline_definitions::Model,
    tasks_definitions: Vec<task_definitions::Model>,
) -> PipelineDefinition {
    let info = pipeline_info_to_domain(pipeline_definition);

    let task_defs: Vec<TaskDefinition> = tasks_definitions
        .into_iter()
        .map(task_definition_to_domain)
        .collect();

    PipelineDefinition {
        info,
        task_definitions: task_defs,
    }
}

pub(crate) fn pipeline_info_to_domain(model: pipeline_definitions::Model) -> PipelineInfo {
    PipelineInfo {
        id: model.id,
        name: model.name.unwrap_or("Unnamed pipeline".to_string()),
        description: model
            .description
            .unwrap_or("No description provided".to_string()),
        metadata: model.metadata,
        default_backend: PipelineBackend::default(),
        default_tags: model.default_tags,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    }
}

pub(crate) fn task_definition_to_domain(model: task_definitions::Model) -> TaskDefinition {
    TaskDefinition {
        id: model.id,
        pipeline_id: model.pipeline_id,
        name: model.name,
        description: model.description,
        config_schema: model.config_schema,
        user_code_metadata: model.user_code_metadata,
        depends_on: model.depends_on.unwrap_or_default(),
    }
}

pub(crate) fn pipeline_run_to_domain(model: pipeline_runs::Model) -> PipelineRun {
    PipelineRun {
        id: model.id,
        definition_id: model.definition_id,
        status: pipeline_status_to_domain(model.status),
        start_time: model.start_time.map(|t| t.into()),
        end_time: model.end_time.map(|t| t.into()),
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    }
}

pub(crate) fn task_instance_to_domain(model: task_instances::Model) -> TaskInstance {
    TaskInstance {
        id: model.id,
        run_id: model.run_id,
        definition_id: model.definition_id,
        status: task_status_to_domain(model.status),
        attempts: model.attempts,
        start_time: model.start_time.map(|t| t.into()),
        end_time: model.end_time.map(|t| t.into()),
        output_metadata: model.output_metadata,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    }
}

pub(crate) fn event_log_to_domain(model: event_log::Model) -> Result<EventLogRecord, Error> {
    let event_type = model.event_type.parse::<EventType>().map_err(|e| {
        Error::Internal(format!(
            "Failed to parse event type '{}': {}",
            model.event_type, e
        ))
    })?;

    Ok(EventLogRecord {
        event_id: model.event_id,
        run_id: model.run_id,
        task_id: model.task_id,
        timestamp: model.timestamp.into(),
        event_type,
        message: model.message,
        metadata: model.metadata,
    })
}

// Implementing From<T> for T is annoying, as the db entities are generated by SeaORM CLI
// and any manually written impls in the `datarouter_state_entity` crate will be overwritten
// on a fresh generation of entities.

pub(crate) fn pipeline_status_to_domain(
    model: sea_orm_active_enums::PipelineRunStatus,
) -> PipelineRunStatus {
    match model {
        sea_orm_active_enums::PipelineRunStatus::Pending => PipelineRunStatus::Pending,
        sea_orm_active_enums::PipelineRunStatus::Queued => PipelineRunStatus::Queued,
        sea_orm_active_enums::PipelineRunStatus::Running => PipelineRunStatus::Running,
        sea_orm_active_enums::PipelineRunStatus::Succeeded => PipelineRunStatus::Succeeded,
        sea_orm_active_enums::PipelineRunStatus::Failed => PipelineRunStatus::Failed,
        sea_orm_active_enums::PipelineRunStatus::Cancelling => PipelineRunStatus::Cancelling,
        sea_orm_active_enums::PipelineRunStatus::Cancelled => PipelineRunStatus::Cancelled,
    }
}

pub(crate) fn domain_pipeline_status_to_db(
    model: PipelineRunStatus,
) -> sea_orm_active_enums::PipelineRunStatus {
    match model {
        PipelineRunStatus::Pending => sea_orm_active_enums::PipelineRunStatus::Pending,
        PipelineRunStatus::Queued => sea_orm_active_enums::PipelineRunStatus::Queued,
        PipelineRunStatus::Running => sea_orm_active_enums::PipelineRunStatus::Running,
        PipelineRunStatus::Succeeded => sea_orm_active_enums::PipelineRunStatus::Succeeded,
        PipelineRunStatus::Failed => sea_orm_active_enums::PipelineRunStatus::Failed,
        PipelineRunStatus::Cancelling => sea_orm_active_enums::PipelineRunStatus::Cancelling,
        PipelineRunStatus::Cancelled => sea_orm_active_enums::PipelineRunStatus::Cancelled,
    }
}

pub(crate) fn pipeline_status_as_expr(model: PipelineRunStatus) -> SimpleExpr {
    Expr::val(model.to_string()).cast_as(Alias::new(PIPELINE_RUN_STATUS_DB_ENUM_NAME))
}

pub(crate) fn task_status_to_domain(model: sea_orm_active_enums::TaskStatus) -> TaskStatus {
    match model {
        sea_orm_active_enums::TaskStatus::Pending => TaskStatus::Pending,
        sea_orm_active_enums::TaskStatus::Queued => TaskStatus::Queued,
        sea_orm_active_enums::TaskStatus::Running => TaskStatus::Running,
        sea_orm_active_enums::TaskStatus::Retrying => TaskStatus::Retrying,
        sea_orm_active_enums::TaskStatus::Succeeded => TaskStatus::Succeeded,
        sea_orm_active_enums::TaskStatus::Failed => TaskStatus::Failed,
        sea_orm_active_enums::TaskStatus::Cancelling => TaskStatus::Cancelling,
        sea_orm_active_enums::TaskStatus::Cancelled => TaskStatus::Cancelled,
        sea_orm_active_enums::TaskStatus::Skipped => TaskStatus::Skipped,
    }
}

pub(crate) fn domain_task_status_to_db(model: TaskStatus) -> sea_orm_active_enums::TaskStatus {
    match model {
        TaskStatus::Pending => sea_orm_active_enums::TaskStatus::Pending,
        TaskStatus::Queued => sea_orm_active_enums::TaskStatus::Queued,
        TaskStatus::Running => sea_orm_active_enums::TaskStatus::Running,
        TaskStatus::Retrying => sea_orm_active_enums::TaskStatus::Retrying,
        TaskStatus::Succeeded => sea_orm_active_enums::TaskStatus::Succeeded,
        TaskStatus::Failed => sea_orm_active_enums::TaskStatus::Failed,
        TaskStatus::Cancelling => sea_orm_active_enums::TaskStatus::Cancelling,
        TaskStatus::Cancelled => sea_orm_active_enums::TaskStatus::Cancelled,
        TaskStatus::Skipped => sea_orm_active_enums::TaskStatus::Skipped,
    }
}

pub(crate) fn task_status_as_expr(model: TaskStatus) -> SimpleExpr {
    Expr::val(model.to_string()).cast_as(Alias::new(TASK_STATUS_DB_ENUM_NAME))
}
