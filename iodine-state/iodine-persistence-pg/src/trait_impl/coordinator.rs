use crate::{
    db::PostgresStateDb,
    entities::{coordinators, launchers},
    event_logging::{log_event_direct, log_event_in_txn},
    mapping::{
        coordinator_status_as_expr, coordinator_to_domain, db_error_to_domain,
        domain_coordinator_status_to_db,
    },
};

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use iodine_common::{
    coordinator::{Coordinator, CoordinatorStatus},
    error::Error,
    event::EventType,
    state::CoordinatorDbTrait,
};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait, prelude::Expr,
};
use uuid::Uuid;

#[async_trait]
impl CoordinatorDbTrait for PostgresStateDb {
    async fn get_coordinator_info(&self) -> Result<Option<Coordinator>, Error> {
        let maybe_coordinator = coordinators::Entity::find_by_id(Uuid::new_v4())
            .one(&self.conn)
            .await
            .map_err(db_error_to_domain)?;

        if let Some(coordinator) = maybe_coordinator {
            Ok(Some(coordinator_to_domain(coordinator)))
        } else {
            Ok(None)
        }
    }

    async fn create_new_coordinator(
        &self,
        id: Uuid,
        hostname: String,
        host_pid: i32,
        version: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let new_coordinator = coordinators::ActiveModel {
            id: Set(id),
            hostname: Set(hostname),
            host_pid: Set(host_pid),
            version: Set(version),
            is_leader: Set(true),
            status: Set(domain_coordinator_status_to_db(CoordinatorStatus::Pending)),
            last_heartbeat: Set(now),
            started_at: Set(now),
            terminated_at: Set(None),
            metadata: Set(metadata),
        };

        let insert_coordinator_res = coordinators::Entity::insert(new_coordinator)
            .exec(&txn)
            .await;

        if let Err(db_err) = insert_coordinator_res {
            txn.rollback().await.map_err(db_error_to_domain)?;

            log_event_direct(
                &self.conn,
                None,
                None,
                EventType::EngineEvent,
                Some(format!("Failed to insert coordinator: {}", db_err)),
                None,
            )
            .await?;

            return Err(db_error_to_domain(db_err));
        }

        txn.commit().await.map_err(db_error_to_domain)?;

        Ok(())
    }

    async fn update_coordinator_status(
        &self,
        id: Uuid,
        status: CoordinatorStatus,
    ) -> Result<(), Error> {
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let mut update_query = coordinators::Entity::update_many()
            .col_expr(
                coordinators::Column::Status,
                coordinator_status_as_expr(status),
            )
            .col_expr(
                coordinators::Column::LastHeartbeat,
                Expr::current_timestamp().into(),
            )
            .filter(coordinators::Column::Id.eq(id));

        match status {
            CoordinatorStatus::Terminated => {
                update_query = update_query.col_expr(
                    coordinators::Column::TerminatedAt,
                    Expr::current_timestamp().into(),
                );
            }
            _ => {
                // TODO(thegenem0):
            }
        }

        let update_res = update_query.exec(&txn).await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => {
                if let Err(db_err) =
                    log_event_in_txn(&txn, Some(id), None, EventType::EngineEvent, None, None).await
                {
                    txn.rollback().await.map_err(db_error_to_domain)?;
                    log_event_direct(
                        &self.conn,
                        Some(id),
                        None,
                        EventType::EngineEvent,
                        Some(format!(
                            "Failed to log coordinator status event for coordinator {}: {}",
                            id, db_err
                        )),
                        None,
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
                    resource_type: "Coordinator".into(),
                    resource_id: id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    Some(id),
                    None,
                    EventType::EngineEvent,
                    Some(format!("Failed to update coordinator status: {}", db_err)),
                    None,
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }

    async fn register_coordinator_heartbeat(&self, id: Uuid) -> Result<(), Error> {
        let update_res = coordinators::Entity::update_many()
            .col_expr(
                coordinators::Column::LastHeartbeat,
                Expr::current_timestamp().into(),
            )
            .filter(coordinators::Column::Id.eq(id))
            .exec(&self.conn)
            .await;

        match update_res {
            Ok(res) if res.rows_affected > 0 => Ok(()),
            Ok(_) => Err(Error::NotFound {
                resource_type: "Coordinator".into(),
                resource_id: id.to_string(),
            }),
            Err(db_err) => Err(db_error_to_domain(db_err)),
        }
    }

    async fn assign_pipeline_to_launcher(
        &self,
        coordinator_id: Uuid,
        launcher_id: Uuid,
        pipeline_id: Uuid,
        _run_id_override: Option<Uuid>,
    ) -> Result<(), Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let insert_launcher_res = launchers::Entity::insert(launchers::ActiveModel {
            id: Set(launcher_id),
            coordinator_id: Set(coordinator_id),
            assigned_pipeline_id: Set(Some(pipeline_id)),
            started_at: Set(Some(now)),
            terminated_at: Set(None),
        })
        .exec(&txn)
        .await;

        if let Err(db_err) = insert_launcher_res {
            txn.rollback().await.map_err(db_error_to_domain)?;

            log_event_direct(
                &self.conn,
                Some(coordinator_id),
                Some(launcher_id),
                EventType::EngineEvent,
                Some(format!("Failed to insert launcher: {}", db_err)),
                None,
            )
            .await?;

            return Err(db_error_to_domain(db_err));
        }

        txn.commit().await.map_err(db_error_to_domain)?;

        Ok(())
    }

    async fn terminate_launcher(
        &self,
        coordinator_id: Uuid,
        launcher_id: Uuid,
    ) -> Result<(), Error> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let txn = self.conn.begin().await.map_err(db_error_to_domain)?;

        let update_launcher_res = launchers::Entity::update_many()
            .col_expr(launchers::Column::TerminatedAt, Expr::value(Some(now)))
            .filter(launchers::Column::Id.eq(launcher_id))
            .filter(launchers::Column::CoordinatorId.eq(coordinator_id))
            .exec(&txn)
            .await;

        match update_launcher_res {
            Ok(res) if res.rows_affected > 0 => {
                if let Err(db_err) = log_event_in_txn(
                    &txn,
                    Some(launcher_id),
                    None,
                    EventType::EngineEvent,
                    None,
                    None,
                )
                .await
                {
                    txn.rollback().await.map_err(db_error_to_domain)?;
                    log_event_direct(
                        &self.conn,
                        Some(launcher_id),
                        None,
                        EventType::EngineEvent,
                        Some(format!(
                            "Failed to log launcher termination event for launcher {}: {}",
                            launcher_id, db_err
                        )),
                        None,
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
                    resource_type: "Launcher".into(),
                    resource_id: launcher_id.to_string(),
                })
            }
            Err(db_err) => {
                txn.rollback().await.map_err(db_error_to_domain)?;

                log_event_direct(
                    &self.conn,
                    Some(launcher_id),
                    None,
                    EventType::EngineEvent,
                    Some(format!("Failed to terminate launcher: {}", db_err)),
                    None,
                )
                .await?;

                Err(db_error_to_domain(db_err))
            }
        }
    }
}
