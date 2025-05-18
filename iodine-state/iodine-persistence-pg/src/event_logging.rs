use chrono::Utc;
use iodine_common::{error::Error, event::EventType};
use sea_orm::{ActiveValue::Set, DatabaseTransaction, DbErr, EntityTrait};
use uuid::Uuid;

use crate::{entities::event_log, mapping::db_error_to_domain};

pub(crate) async fn log_event_in_txn(
    txn: &DatabaseTransaction,
    pipeline_run_id: Option<Uuid>,
    task_run_id: Option<Uuid>,
    event_type: EventType,
    message: Option<String>,
    metadata: Option<serde_json::Value>,
) -> Result<(), DbErr> {
    let event = event_log::ActiveModel {
        pipeline_run_id: Set(pipeline_run_id),
        task_run_id: Set(task_run_id),
        event_type: Set(event_type.to_string()),
        message: Set(message),
        metadata: Set(metadata),
        timestamp: Set(Utc::now().into()),
        ..Default::default()
    };

    event_log::Entity::insert(event).exec(txn).await?;

    Ok(())
}

pub(crate) async fn log_event_direct(
    conn: &sea_orm::DatabaseConnection,
    pipeline_run_id: Option<Uuid>,
    task_run_id: Option<Uuid>,
    event_type: EventType,
    message: Option<String>,
    metadata: Option<serde_json::Value>,
) -> Result<(), Error> {
    let event = event_log::ActiveModel {
        pipeline_run_id: Set(pipeline_run_id),
        task_run_id: Set(task_run_id),
        event_type: Set(event_type.to_string()),
        message: Set(message),
        metadata: Set(metadata),
        timestamp: Set(Utc::now().into()),
        ..Default::default()
    };

    event_log::Entity::insert(event)
        .exec(conn)
        .await
        .map_err(db_error_to_domain)?;

    Ok(())
}
