use sea_orm_migration::sea_query;
use sea_orm_migration::{
    prelude::*,
    sea_orm::{DeriveActiveEnum, EnumIter},
};

#[derive(Iden)]
#[iden = "coordinators"]
pub enum Coordinator {
    Table,
    #[iden = "id"]
    Id,
    #[iden = "hostname"]
    Hostname,
    #[iden = "host_pid"]
    HostPid,
    #[iden = "is_leader"]
    IsLeader,
    #[iden = "status"]
    Status,
    #[iden = "version"]
    Version,
    #[iden = "last_heartbeat"]
    LastHeartbeat,
    #[iden = "started_at"]
    StartedAt,
    #[iden = "terminated_at"]
    TerminatedAt,
    #[iden = "metadata"]
    Metadata,
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "coordinator_status")]
pub enum DbCoordinatorStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "RUNNING")]
    Running,
    #[sea_orm(string_value = "TERMINATING")]
    Terminating,
    #[sea_orm(string_value = "TERMINATED")]
    Terminated,
}
