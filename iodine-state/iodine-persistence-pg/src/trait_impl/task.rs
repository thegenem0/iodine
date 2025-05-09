use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use iodine_common::{
    error::Error,
    event::EventType,
    state::TaskDbTrait,
    task::{TaskDefinition, TaskInstance, TaskStatus},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait, prelude::Expr};
use uuid::Uuid;

use crate::{
    db::PostgresStateDb,
    entities::{pipeline_runs, task_definitions, task_dependencies, task_instances},
    event_logging::{log_event_direct, log_event_in_txn},
    mapping::{
        db_error_to_domain, domain_task_status_to_db, task_definition_to_domain,
        task_instance_to_domain, task_status_as_expr, task_status_to_domain,
    },
};

#[async_trait]
#[allow(unused_variables)]
impl TaskDbTrait for PostgresStateDb {
    async fn get_task_definition(
        &self,
        definition_id: Uuid,
    ) -> Result<Option<TaskDefinition>, Error> {
        let maybe_model = task_definitions::Entity::find_by_id(definition_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        Ok(maybe_model.map(task_definition_to_domain))
    }

    async fn list_task_definitions(
        &self,
        pipeline_id: Option<Uuid>,
    ) -> Result<(Vec<TaskDefinition>, u64), Error> {
        let mut query = task_definitions::Entity::find();

        if let Some(pipeline_id) = pipeline_id {
            query = query.filter(task_definitions::Column::PipelineId.eq(pipeline_id));
        }

        let definitions = query.all(&self.conn).await.map_err(db_error_to_domain)?;

        let total_count = definitions.len();

        Ok((
            definitions
                .into_iter()
                .map(task_definition_to_domain)
                .collect(),
            total_count as u64,
        ))
    }

    async fn get_task_instance(&self, task_id: Uuid) -> Result<Option<TaskInstance>, Error> {
        let maybe_model = task_instances::Entity::find_by_id(task_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        Ok(maybe_model.map(task_instance_to_domain))
    }

    async fn list_task_instances(
        &self,
        run_id: Option<Uuid>,
    ) -> Result<(Vec<TaskInstance>, u64), Error> {
        let mut query = task_instances::Entity::find();

        if let Some(run_id) = run_id {
            query = query.filter(task_instances::Column::RunId.eq(run_id));
        }

        let instances = query.all(&self.conn).await.map_err(db_error_to_domain)?;

        let total_count = instances.len();

        Ok((
            instances.into_iter().map(task_instance_to_domain).collect(),
            total_count as u64,
        ))
    }

    async fn get_prerequisite_statuses(
        &self,
        run_id: Uuid,
        task_id: Uuid,
    ) -> Result<HashMap<Uuid, TaskStatus>, Error> {
        let target_task =
            self.get_task_instance(task_id)
                .await?
                .ok_or_else(|| Error::NotFound {
                    resource_type: "TaskInstance".into(),
                    resource_id: task_id.to_string(),
                })?;

        if target_task.run_id != run_id {
            return Err(Error::InvalidInput(format!(
                "Target task instance {} does not belong to run: {}",
                task_id, run_id
            )));
        }

        let dag_run_model = pipeline_runs::Entity::find_by_id(run_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?
            .ok_or_else(|| Error::NotFound {
                resource_type: "PipelineRun".into(),
                resource_id: run_id.to_string(),
            })?;

        let source_deps = task_dependencies::Entity::find()
            .filter(task_dependencies::Column::TargetTaskDefinitionId.eq(target_task.definition_id))
            .filter(task_dependencies::Column::PipelineId.eq(dag_run_model.definition_id))
            .all(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        let source_dep_ids: Vec<Uuid> = source_deps
            .into_iter()
            .map(|model| model.source_task_definition_id)
            .collect();

        if source_dep_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let prereq_statuses = task_instances::Entity::find()
            .filter(task_instances::Column::RunId.eq(target_task.run_id))
            .filter(task_instances::Column::DefinitionId.is_in(source_dep_ids))
            .all(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        // Map Task_id -> Status
        let res = prereq_statuses
            .into_iter()
            .map(|model| (model.id, task_status_to_domain(model.status)))
            .collect();

        Ok(res)
    }

    async fn update_task_status(
        &self,
        task_id: Uuid,
        new_status: TaskStatus,
        metadata: Option<serde_json::Value>,
        error_data: Option<serde_json::Value>,
    ) -> Result<(), Error> {
        let task_instance = task_instances::Entity::find_by_id(task_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?
            .ok_or_else(|| Error::NotFound {
                resource_type: "TaskInstance".into(),
                resource_id: task_id.to_string(),
            })?;

        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let mut update_query = task_instances::Entity::update_many()
            .col_expr(
                task_instances::Column::Status,
                task_status_as_expr(new_status),
            )
            .col_expr(task_instances::Column::UpdatedAt, Expr::value(Some(now)))
            .col_expr(
                task_instances::Column::EndTime,
                Expr::case(
                    Expr::value(matches!(
                        new_status,
                        TaskStatus::Succeeded
                            | TaskStatus::Failed
                            | TaskStatus::Cancelled
                            | TaskStatus::Skipped
                    )),
                    Expr::value(Some(now)),
                )
                .finally(Expr::col(task_instances::Column::EndTime))
                .into(),
            )
            .filter(task_instances::Column::Id.eq(task_id));

        if let Some(error_data) = error_data {
            update_query = update_query.col_expr(
                task_instances::Column::ErrorData,
                Expr::value(Some(error_data.clone())),
            );
        }

        match new_status {
            TaskStatus::Queued => {
                // Only pending tasks can be queued
                update_query = update_query.filter(
                    task_instances::Column::Status
                        .eq(domain_task_status_to_db(TaskStatus::Pending)),
                );
            }
            TaskStatus::Succeeded => {
                update_query = update_query
                    .col_expr(task_instances::Column::EndTime, Expr::value(Some(now)))
                    .col_expr(
                        task_instances::Column::OutputMetadata,
                        Expr::value(metadata.clone()),
                    )
                    // Only running tasks can succeed
                    .filter(
                        task_instances::Column::Status
                            .eq(domain_task_status_to_db(TaskStatus::Running)),
                    );
            }
            TaskStatus::Skipped => {
                // Only pending tasks can be skipped
                update_query = update_query.filter(
                    task_instances::Column::Status
                        .eq(domain_task_status_to_db(TaskStatus::Pending)),
                );
            }
            TaskStatus::Retrying => {
                update_query = update_query
                    .col_expr(
                        task_instances::Column::StartTime,
                        Expr::value(None::<DateTime<Utc>>),
                    )
                    .col_expr(
                        task_instances::Column::EndTime,
                        Expr::value(None::<DateTime<Utc>>),
                    )
                    .col_expr(task_instances::Column::WorkerId, Expr::value(None::<Uuid>))
                    // Only failed or cancelled tasks can be retried
                    .filter(
                        task_instances::Column::Status
                            .eq(domain_task_status_to_db(TaskStatus::Failed)),
                    )
                    .filter(
                        task_instances::Column::Status
                            .eq(domain_task_status_to_db(TaskStatus::Cancelled)),
                    );
            }
            _ => unreachable!("Cannot update task status to {:?}", new_status),
        }

        let update_res = update_query.exec(&txn).await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                if let Err(log_err) = log_event_in_txn(
                    &txn,
                    Some(task_instance.run_id),
                    Some(task_instance.id),
                    EventType::TaskSuccess,
                    None,
                    metadata,
                )
                .await
                {
                    txn.rollback().await.map_err(db_error_to_domain)?;

                    log_event_direct(
                        &self.conn,
                        Some(task_instance.run_id),
                        Some(task_instance.id),
                        EventType::EngineEvent,
                        Some(format!(
                            "Failed to log event: {}, error: {}",
                            new_status, log_err
                        )),
                        Some(serde_json::json!({"action": "record_task_status"})),
                    )
                    .await?;

                    return Err(db_error_to_domain(log_err));
                }

                txn.commit().await.map_err(db_error_to_domain)?;

                Ok(())
            }
            Ok(_) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                Err(Error::NotFound {
                    resource_type: "TaskInstance".into(),
                    resource_id: task_instance.id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    Some(task_instance.run_id),
                    Some(task_instance.id),
                    EventType::EngineEvent,
                    Some(format!(
                        "Failed DB update for task event: {}, error: {}",
                        new_status, db_err
                    )),
                    Some(serde_json::json!({"action": "record_task_status"})),
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }

    async fn enqueue_task(&self, task_id: Uuid) -> Result<(), Error> {
        self.update_task_status(task_id, TaskStatus::Queued, None, None)
            .await
    }

    async fn record_task_heartbeat(&self, task_id: Uuid) -> Result<(), Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let update_res = task_instances::Entity::update_many()
            .col_expr(
                task_instances::Column::LastHeartbeat,
                Expr::value(Some(now)),
            )
            .filter(task_instances::Column::Id.eq(task_id))
            .exec(&txn)
            .await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                txn.commit().await.map_err(db_error_to_domain)?;

                Ok(())
            }
            Ok(_) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                Err(Error::NotFound {
                    resource_type: "TaskInstance".into(),
                    resource_id: task_id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    None,
                    None,
                    EventType::EngineEvent,
                    Some(format!("Failed DB update for task heartbeat: {}", db_err)),
                    Some(serde_json::json!({"action": "record_task_heartbeat"})),
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }

    async fn claim_task(&self, task_id: Uuid, worker_id: Uuid) -> Result<TaskInstance, Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let update_res = task_instances::Entity::update_many()
            .col_expr(
                task_instances::Column::WorkerId,
                Expr::value(Some(worker_id)),
            )
            .col_expr(task_instances::Column::StartTime, Expr::value(Some(now)))
            // Null out the end time, since a new worker is claiming the task
            // This might already be populated if the task is retried
            .col_expr(
                task_instances::Column::EndTime,
                Expr::value(None::<DateTime<Utc>>),
            )
            // Null out the last heartbeat, since a new worker is claiming the task
            // This might already be populated if the task is retried
            .col_expr(
                task_instances::Column::LastHeartbeat,
                Expr::value(None::<DateTime<Utc>>),
            )
            .filter(task_instances::Column::Id.eq(task_id))
            .exec(&txn)
            .await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                let model = task_instances::Entity::find_by_id(task_id)
                    .one(&self.conn)
                    .await
                    .map_err(db_error_to_domain)?
                    .ok_or_else(|| Error::NotFound {
                        resource_type: "TaskInstance".into(),
                        resource_id: task_id.to_string(),
                    })?;

                txn.commit().await.map_err(db_error_to_domain)?;

                Ok(task_instance_to_domain(model))
            }
            Ok(_) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                Err(Error::NotFound {
                    resource_type: "TaskInstance".into(),
                    resource_id: task_id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    None,
                    None,
                    EventType::EngineEvent,
                    Some(format!("Failed DB update for task claim: {}", db_err)),
                    Some(serde_json::json!({"action": "claim_task"})),
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }
}
