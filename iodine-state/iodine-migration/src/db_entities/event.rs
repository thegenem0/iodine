use sea_orm_migration::sea_orm::Iden;
use sea_orm_migration::sea_query;

#[derive(Iden)]
#[iden = "event_log"]
pub enum EventLog {
    Table,
    #[iden = "event_id"]
    EventId,
    #[iden = "run_id"]
    RunId,
    #[iden = "task_id"]
    TaskId,
    #[iden = "timestamp"]
    Timestamp,
    #[iden = "event_type"]
    EventType,
    #[iden = "message"]
    Message,
    #[iden = "metadata"]
    Metadata,
}
