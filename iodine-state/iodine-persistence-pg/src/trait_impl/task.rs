use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use iodine_common::{
    error::Error,
    event::EventType,
    state::TaskDbTrait,
    task::{TaskDefinition, TaskRun, TaskRunStatus},
};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait, prelude::Expr,
};
use uuid::Uuid;

use crate::{
    db::PostgresStateDb,
    entities::{task_definitions, task_runs},
    event_logging::{log_event_direct, log_event_in_txn},
    mapping::{
        db_error_to_domain, domain_task_status_to_db, task_definition_to_domain,
        task_run_to_domain, task_status_as_expr,
    },
};

#[async_trait]
#[allow(unused_variables)]
impl TaskDbTrait for PostgresStateDb {
    async fn get_task_definition(
        &self,
        task_def_id: Uuid,
    ) -> Result<Option<TaskDefinition>, Error> {
        let maybe_model = task_definitions::Entity::find_by_id(task_def_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        let model = maybe_model.map(task_definition_to_domain).transpose()?;

        Ok(model)
    }

    async fn list_task_definitions(
        &self,
        pipeline_def_id: Option<Uuid>,
    ) -> Result<(Vec<TaskDefinition>, u64), Error> {
        let mut query = task_definitions::Entity::find();

        if let Some(def_id) = pipeline_def_id {
            query = query.filter(task_definitions::Column::PipelineDefId.eq(def_id));
        }

        let definitions = query.all(&self.conn).await.map_err(db_error_to_domain)?;

        let total_count = definitions.len();

        let mut models = Vec::new();

        for def in definitions {
            let model = task_definition_to_domain(def)?;
            models.push(model);
        }

        Ok((models, total_count as u64))
    }

    async fn get_task_run(&self, task_run_id: Uuid) -> Result<Option<TaskRun>, Error> {
        let maybe_model = task_runs::Entity::find_by_id(task_run_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        Ok(maybe_model.map(task_run_to_domain))
    }

    async fn list_task_runs(
        &self,
        pipeline_run_id: Option<Uuid>,
    ) -> Result<(Vec<TaskRun>, u64), Error> {
        let mut query = task_runs::Entity::find();

        if let Some(run_id) = pipeline_run_id {
            query = query.filter(task_runs::Column::PipelineRunId.eq(run_id));
        }

        let instances = query.all(&self.conn).await.map_err(db_error_to_domain)?;

        let total_count = instances.len();

        Ok((
            instances.into_iter().map(task_run_to_domain).collect(),
            total_count as u64,
        ))
    }

    async fn update_task_run_status(
        &self,
        task_run_id: Uuid,
        new_status: TaskRunStatus,
        metadata: Option<serde_json::Value>,
        error_data: Option<serde_json::Value>,
    ) -> Result<(), Error> {
        let task_run = task_runs::Entity::find_by_id(task_run_id)
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?
            .ok_or_else(|| Error::NotFound {
                resource_type: "TaskInstance".into(),
                resource_id: task_run_id.to_string(),
            })?;

        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let mut update_query = task_runs::Entity::update_many()
            .col_expr(task_runs::Column::Status, task_status_as_expr(new_status))
            .col_expr(task_runs::Column::UpdatedAt, Expr::value(Some(now)))
            .col_expr(
                task_runs::Column::EndTime,
                Expr::case(
                    Expr::value(matches!(
                        new_status,
                        TaskRunStatus::Succeeded
                            | TaskRunStatus::Failed
                            | TaskRunStatus::Cancelled
                            | TaskRunStatus::Skipped
                    )),
                    Expr::value(Some(now)),
                )
                .finally(Expr::col(task_runs::Column::EndTime))
                .into(),
            )
            .filter(task_runs::Column::Id.eq(task_run_id));

        match new_status {
            TaskRunStatus::Queued => {
                // Only pending tasks can be queued
                update_query = update_query.filter(
                    task_runs::Column::Status.eq(domain_task_status_to_db(TaskRunStatus::Pending)),
                );
            }
            TaskRunStatus::Succeeded => {
                update_query = update_query
                    .col_expr(
                        task_runs::Column::OutputMetadata,
                        Expr::value(metadata.clone()),
                    )
                    // Only running tasks can succeed
                    .filter(
                        task_runs::Column::Status
                            .eq(domain_task_status_to_db(TaskRunStatus::Running)),
                    );
            }
            TaskRunStatus::Skipped => {
                // Only pending tasks can be skipped
                update_query = update_query.filter(
                    task_runs::Column::Status.eq(domain_task_status_to_db(TaskRunStatus::Pending)),
                );
            }
            TaskRunStatus::Retrying => {
                update_query = update_query
                    .col_expr(
                        task_runs::Column::StartTime,
                        Expr::value(None::<DateTime<Utc>>),
                    )
                    .col_expr(
                        task_runs::Column::EndTime,
                        Expr::value(None::<DateTime<Utc>>),
                    )
                    .filter(
                        task_runs::Column::Status
                            .eq(domain_task_status_to_db(TaskRunStatus::Failed)),
                    )
                    .filter(
                        task_runs::Column::Status
                            .eq(domain_task_status_to_db(TaskRunStatus::Cancelled)),
                    );
            }
            TaskRunStatus::Running => {
                update_query =
                    update_query.col_expr(task_runs::Column::StartTime, Expr::value(Some(now)));
            }
            _ => { /* No additional fields to update */ }
        }

        let update_res = update_query.exec(&txn).await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                if let Err(log_err) = log_event_in_txn(
                    &txn,
                    Some(task_run.pipeline_run_id),
                    Some(task_run.id),
                    EventType::TaskSuccess,
                    None,
                    metadata,
                )
                .await
                {
                    txn.rollback().await.map_err(db_error_to_domain)?;

                    log_event_direct(
                        &self.conn,
                        Some(task_run.pipeline_run_id),
                        Some(task_run.id),
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
                    resource_id: task_run.id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    Some(task_run.pipeline_run_id),
                    Some(task_run.id),
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

    async fn create_task_run(
        &self,
        task_run_id: Uuid,
        task_def_id: Uuid,
        pipeline_run_id: Uuid,
        attempt: i32,
        status: TaskRunStatus,
        output: Option<serde_json::Value>,
        message: Option<String>,
    ) -> Result<(), Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let new_task_run = task_runs::ActiveModel {
            id: Set(task_run_id),
            task_def_id: Set(task_def_id),
            pipeline_run_id: Set(pipeline_run_id),
            status: Set(domain_task_status_to_db(status)),
            attempts: Set(attempt),
            output_metadata: Set(output),
            message: Set(message),
            start_time: Set(Some(now)),
            end_time: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let insert_res = task_runs::Entity::insert(new_task_run).exec(&txn).await;

        if let Err(db_err) = insert_res {
            txn.rollback().await.map_err(db_error_to_domain)?;

            log_event_direct(
                &self.conn,
                Some(task_run_id),
                None,
                EventType::EngineEvent,
                Some(format!("Failed to insert task run: {}", db_err)),
                None,
            )
            .await?;

            return Err(db_error_to_domain(db_err));
        }

        txn.commit().await.map_err(db_error_to_domain)?;

        Ok(())
    }
}
