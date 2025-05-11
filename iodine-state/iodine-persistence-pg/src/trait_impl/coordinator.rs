use crate::{
    db::PostgresStateDb,
    entities::coordinators,
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

        let update_res = coordinators::Entity::update_many()
            .col_expr(
                coordinators::Column::Status,
                coordinator_status_as_expr(status),
            )
            .col_expr(
                coordinators::Column::LastHeartbeat,
                Expr::current_timestamp().into(),
            )
            .filter(coordinators::Column::Id.eq(id))
            .exec(&txn)
            .await;

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
        _launcher_id: Uuid,
        _pipeline_id: Uuid,
        _run_id_override: Option<Uuid>,
    ) -> Result<(), Error> {
        todo!()
    }
}
