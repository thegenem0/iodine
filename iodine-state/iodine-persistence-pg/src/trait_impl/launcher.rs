use crate::db::PostgresStateDb;

use async_trait::async_trait;

use iodine_common::state::LauncherDbTrait;

#[async_trait]
impl LauncherDbTrait for PostgresStateDb {}
