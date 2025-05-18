use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use iodine_common::error::WorkerError;
use iodine_common::event::EventType;
use iodine_common::pipeline::PipelineInfo;
use iodine_common::{
    error::Error,
    pipeline::{PipelineDefinition, PipelineRun, PipelineRunStatus},
    state::PipelineDbTrait,
};
use sea_orm::ActiveValue::{self, Set};
use sea_orm::prelude::Expr;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

use crate::db::PostgresStateDb;
use crate::entities::{pipeline_definitions, pipeline_runs, task_definitions};
use crate::event_logging::{log_event_direct, log_event_in_txn};
use crate::mapping::{
    db_error_to_domain, domain_pipeline_status_to_db, pipeline_definition_to_domain,
    pipeline_info_to_domain, pipeline_run_to_domain, pipeline_status_as_expr,
    pipeline_status_to_domain,
};

#[async_trait]
#[allow(unused_variables)]
impl PipelineDbTrait for PostgresStateDb {
    async fn get_pipeline_definition(
        &self,
        pipeline_def_id: Uuid,
    ) -> Result<Option<PipelineDefinition>, Error> {
        let maybe_pipeline_def = pipeline_definitions::Entity::find_by_id(pipeline_def_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        let task_defs = task_definitions::Entity::find()
            .filter(task_definitions::Column::PipelineDefId.eq(pipeline_def_id))
            .all(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        if let Some(pipeline_def) = maybe_pipeline_def {
            let def = pipeline_definition_to_domain(pipeline_def, task_defs)?;

            Ok(Some(def))
        } else {
            Ok(None)
        }
    }

    async fn list_pipeline_definitions(&self) -> Result<(Vec<PipelineInfo>, u64), Error> {
        let pipeline_defs = pipeline_definitions::Entity::find()
            .all(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        let total_count = pipeline_defs.len();

        Ok((
            pipeline_defs
                .into_iter()
                .map(pipeline_info_to_domain)
                .collect(),
            total_count as u64,
        ))
    }

    async fn get_pipeline_run(&self, pipeline_run_id: Uuid) -> Result<Option<PipelineRun>, Error> {
        let maybe_pipeline_run = pipeline_runs::Entity::find_by_id(pipeline_run_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        Ok(maybe_pipeline_run.map(pipeline_run_to_domain))
    }

    async fn get_active_runs(&self) -> Result<HashMap<Uuid, PipelineRunStatus>, Error> {
        let active_runs = pipeline_runs::Entity::find()
            .filter(
                pipeline_runs::Column::Status
                    .eq(domain_pipeline_status_to_db(PipelineRunStatus::Running)),
            )
            .filter(
                pipeline_runs::Column::Status
                    .eq(domain_pipeline_status_to_db(PipelineRunStatus::Queued)),
            )
            .all(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        Ok(active_runs
            .into_iter()
            .map(|run| (run.id, pipeline_status_to_domain(run.status)))
            .collect())
    }

    async fn list_runs(&self) -> Result<(Vec<PipelineRun>, u64), Error> {
        let runs = pipeline_runs::Entity::find()
            .all(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        let total_count = runs.len();

        Ok((
            runs.into_iter().map(pipeline_run_to_domain).collect(),
            total_count as u64,
        ))
    }

    async fn register_pipeline(
        &self,
        pipeline_definition: &PipelineDefinition,
    ) -> Result<(), Error> {
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;
        let def_id = pipeline_definition.info.id;

        task_definitions::Entity::delete_many()
            .filter(task_definitions::Column::PipelineDefId.eq(def_id))
            .exec(&txn)
            .await
            .map_err(db_error_to_domain)?;

        let def_active_model = pipeline_definitions::ActiveModel {
            id: Set(pipeline_definition.info.id),
            name: Set(Some(pipeline_definition.info.name.clone())),
            description: Set(Some(pipeline_definition.info.description.clone())),
            metadata: Set(pipeline_definition.info.metadata.clone()),
            created_at: ActiveValue::NotSet,
            updated_at: Set(Utc::now().into()),
            default_backend: Set(Some(pipeline_definition.info.default_backend.to_string())),
            default_tags: Set(pipeline_definition.info.default_tags.clone()),
        };

        let update_event: EventType;

        match pipeline_definitions::Entity::find_by_id(def_id)
            .one(&txn)
            .await
            .map_err(db_error_to_domain)?
        {
            Some(_) => {
                update_event = EventType::DefinitionUpdated;

                pipeline_definitions::Entity::update(def_active_model)
                    .filter(pipeline_definitions::Column::Id.eq(def_id))
                    .exec(&txn)
                    .await
                    .map_err(db_error_to_domain)?;
            }
            None => {
                update_event = EventType::DefinitionRegistered;

                pipeline_definitions::Entity::insert(def_active_model)
                    .exec(&txn)
                    .await
                    .map_err(db_error_to_domain)?;
            }
        };

        let taks_inserts: Vec<task_definitions::ActiveModel> = pipeline_definition
            .task_definitions
            .iter()
            .map(|task| {
                let execution_ctx: serde_json::Value =
                    task.execution_ctx.clone().try_into().unwrap_or_default();

                return task_definitions::ActiveModel {
                    id: Set(task.id),
                    pipeline_def_id: Set(task.pipeline_def_id),
                    name: Set(task.name.clone()),
                    description: Set(task.description.clone()),
                    execution_context: Set(execution_ctx),
                    max_attempts: Set(task.max_attempts.map(|m| m as i32)),
                    depends_on: Set(Some(task.depends_on.clone())),
                };
            })
            .collect();

        if !taks_inserts.is_empty() {
            task_definitions::Entity::insert_many(taks_inserts)
                .exec(&txn)
                .await
                .map_err(db_error_to_domain)?;
        }

        let message = format!(
            "Registered pipeline definition with id {} and {} nodes",
            pipeline_definition.info.id,
            pipeline_definition.task_definitions.len(),
        );

        log_event_in_txn(&txn, None, None, update_event, Some(message), None)
            .await
            .map_err(db_error_to_domain)?;

        txn.commit().await.map_err(db_error_to_domain)?;

        Ok(())
    }

    async fn deregister_pipeline(&self, pipeline_def_id: Uuid) -> Result<(), Error> {
        // TODO(thegenem0):
        // probably don't want to actually delete these records.
        // could be used for historical viewing, maybe just mark them as deregistered?

        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let pipeline_def = pipeline_definitions::Entity::find_by_id(pipeline_def_id)
            .one(&txn)
            .await
            .map_err(db_error_to_domain)?;

        if let Some(pipeline_def) = pipeline_def {
            task_definitions::Entity::delete_many()
                .filter(task_definitions::Column::PipelineDefId.eq(pipeline_def_id))
                .exec(&txn)
                .await
                .map_err(db_error_to_domain)?;

            pipeline_definitions::Entity::delete_by_id(pipeline_def_id)
                .exec(&txn)
                .await
                .map_err(db_error_to_domain)?;
        }

        let message = format!(
            "Deregistered pipeline definition with id {}",
            pipeline_def_id
        );

        log_event_in_txn(
            &txn,
            None,
            None,
            EventType::DefinitionDeregistered,
            Some(message),
            None,
        )
        .await
        .map_err(db_error_to_domain)?;

        txn.commit().await.map_err(db_error_to_domain)?;

        Ok(())
    }

    async fn create_pipeline_run(
        &self,
        pipeline_run_id: Uuid,
        pipeline_def_id: Uuid,
        launcher_id: Uuid,
        initial_run_status: PipelineRunStatus,
    ) -> Result<(), Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let run_id = Uuid::new_v4();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let new_run = pipeline_runs::ActiveModel {
            id: Set(run_id),
            pipeline_def_id: Set(pipeline_def_id),
            launcher_id: Set(launcher_id),
            status: Set(domain_pipeline_status_to_db(PipelineRunStatus::Queued)),
            ..Default::default()
        };

        let insert_run_res = pipeline_runs::Entity::insert(new_run).exec(&txn).await;

        if let Err(db_err) = insert_run_res {
            txn.rollback().await.map_err(db_error_to_domain)?;

            log_event_direct(
                &self.conn,
                Some(run_id),
                None,
                EventType::RunFailure,
                Some(format!(
                    "Failed run creation: insert dag_runs failed: {}",
                    db_err
                )),
                None,
            )
            .await?;

            return Err(db_error_to_domain(db_err));
        }

        let run_event_type = match initial_run_status {
            PipelineRunStatus::Queued => EventType::RunEnqueued,
            PipelineRunStatus::Pending => EventType::RunStarting,
            _ => EventType::RunStart,
        };

        if let Err(db_err) =
            log_event_in_txn(&txn, Some(run_id), None, run_event_type, None, None).await
        {
            txn.rollback().await.map_err(db_error_to_domain)?;
            log_event_direct(
                &self.conn,
                Some(run_id),
                None,
                EventType::RunFailure,
                Some(format!("Failed run creation: log event failed: {}", db_err)),
                None,
            )
            .await?;
            return Err(db_error_to_domain(db_err));
        }

        txn.commit().await.map_err(db_error_to_domain)?;

        Ok(())
    }

    async fn update_pipeline_run_status(
        &self,
        pipeline_run_id: Uuid,
        new_status: PipelineRunStatus,
        message: Option<String>,
    ) -> Result<(), Error> {
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let event_type = match new_status {
            PipelineRunStatus::Queued => EventType::RunEnqueued,
            PipelineRunStatus::Pending => EventType::RunStarting,
            PipelineRunStatus::Running => EventType::RunStart,
            PipelineRunStatus::Succeeded => EventType::RunSuccess,
            PipelineRunStatus::Failed => EventType::RunFailure,
            PipelineRunStatus::Cancelling => EventType::RunCanceling,
            PipelineRunStatus::Cancelled => EventType::RunCanceled,
        };

        let update_res = pipeline_runs::Entity::update_many()
            .col_expr(
                pipeline_runs::Column::Status,
                pipeline_status_as_expr(PipelineRunStatus::Queued),
            )
            .col_expr(
                pipeline_runs::Column::UpdatedAt,
                Expr::value(Some(Utc::now())),
            )
            .filter(pipeline_runs::Column::Id.eq(pipeline_run_id))
            .exec(&txn)
            .await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                if let Err(db_err) =
                    log_event_in_txn(&txn, Some(pipeline_run_id), None, event_type, message, None)
                        .await
                {
                    txn.rollback().await.map_err(db_error_to_domain)?;

                    log_event_direct(
                        &self.conn,
                        Some(pipeline_run_id),
                        None,
                        EventType::EngineEvent,
                        Some(format!(
                            "Failed to log enqueue event for run{}: {}",
                            pipeline_run_id, db_err
                        )),
                        Some(serde_json::json!({"action": "record_run_enqueued"})),
                    )
                    .await?;

                    return Err(db_error_to_domain(db_err));
                }

                txn.commit().await.map_err(db_error_to_domain)?;

                return Ok(());
            }
            Ok(_) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                Err(Error::NotFound {
                    resource_type: "DagRun".into(),
                    resource_id: pipeline_run_id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    Some(pipeline_run_id),
                    None,
                    EventType::EngineEvent,
                    Some(format!("Failed DB update for run enqueue: {}", db_err)),
                    Some(serde_json::json!({"action": "record_run_enqueued"})),
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }

    async fn finalize_pipeline_run(
        &self,
        pipeline_run_id: Uuid,
        final_status: PipelineRunStatus,
        error_info: Option<&WorkerError>,
    ) -> Result<(), Error> {
        if !matches!(
            final_status,
            PipelineRunStatus::Succeeded | PipelineRunStatus::Failed | PipelineRunStatus::Cancelled
        ) {
            return Err(Error::InvalidInput(
                "Final status must be Succeeded, Failed, or Cancelled".into(),
            ));
        }

        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let update_res = pipeline_runs::Entity::update_many()
            .col_expr(
                pipeline_runs::Column::Status,
                pipeline_status_as_expr(final_status),
            )
            .col_expr(pipeline_runs::Column::EndTime, Expr::value(Some(now)))
            .col_expr(pipeline_runs::Column::UpdatedAt, Expr::value(Some(now)))
            .filter(pipeline_runs::Column::Id.eq(pipeline_run_id))
            .filter(
                Condition::any()
                    .add(
                        pipeline_runs::Column::Status
                            .eq(domain_pipeline_status_to_db(PipelineRunStatus::Running)),
                    )
                    .add(
                        pipeline_runs::Column::Status
                            .eq(domain_pipeline_status_to_db(PipelineRunStatus::Cancelled)),
                    ),
            )
            .exec(&txn)
            .await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                let (event_type, event_metadata) = match final_status {
                    PipelineRunStatus::Succeeded => (EventType::RunSuccess, None),
                    PipelineRunStatus::Failed => {
                        (EventType::RunFailure, serde_json::to_value(error_info).ok())
                    }
                    PipelineRunStatus::Cancelled => (EventType::RunCanceled, None),
                    _ => unreachable!(),
                };

                if let Err(db_err) = log_event_in_txn(
                    &txn,
                    Some(pipeline_run_id),
                    None,
                    event_type,
                    None,
                    event_metadata,
                )
                .await
                {
                    txn.rollback().await.map_err(db_error_to_domain)?;
                    log_event_direct(
                        &self.conn,
                        Some(pipeline_run_id),
                        None,
                        EventType::EngineEvent,
                        Some(format!(
                            "Failed to log status event for run {}: {}",
                            pipeline_run_id, db_err
                        )),
                        Some(serde_json::json!({"action": "record_run_status"})),
                    )
                    .await?;
                    return Err(db_error_to_domain(db_err));
                }

                txn.commit().await.map_err(db_error_to_domain)?;

                return Ok(());
            }
            Ok(_) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                Err(Error::StateTransition(format!(
                    "Run {} not found or not in Running/Canceling state for final update",
                    pipeline_run_id
                )))
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    Some(pipeline_run_id),
                    None,
                    EventType::EngineEvent,
                    Some(format!("Failed DB update for run status: {}", db_err)),
                    Some(serde_json::json!({"action": "record_run_status"})),
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }
}
