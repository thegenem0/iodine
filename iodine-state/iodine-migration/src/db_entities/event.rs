use sea_orm_migration::sea_orm::Iden;
use sea_orm_migration::sea_query;

#[derive(Iden)]
#[iden = "event_log"]
pub enum EventLog {
    Table,
    #[iden = "event_id"]
    EventId,
    #[iden = "pipeline_run_id"]
    PipelineRunId,
    #[iden = "task_run_id"]
    TaskRunId,
    #[iden = "timestamp"]
    Timestamp,
    #[iden = "event_type"]
    EventType,
    #[iden = "message"]
    Message,
    #[iden = "metadata"]
    Metadata,
}
