use iodine_common::{error::Error, state::DatabaseTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

#[derive(Debug, Clone)]
pub struct PostgresStateDb {
    pub(crate) conn: DatabaseConnection,
}

impl PostgresStateDb {
    pub async fn new(db_url: &str) -> Result<Self, Error> {
        let mut opt = ConnectOptions::new(db_url.to_string());
        opt.sqlx_logging(false);

        let conn = Database::connect(opt)
            .await
            .map_err(|e| Error::Database(format!("Failed to connect to database: {e}")))?;

        Ok(Self { conn })
    }
}

/// Implements the [DatabaseTrait] for [PostgresStateDb]
/// This is a wrapper trait around several
/// Iodine DB traits, each for a specific purpose.
/// Let's us do some fun and easy dyn dispatch with
/// the actual database implementation.
impl DatabaseTrait for PostgresStateDb {}
