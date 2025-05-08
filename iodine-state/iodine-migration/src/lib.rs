pub use sea_orm_migration::prelude::*;

mod db_entities;
mod migrations;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            migrations::m20220101_000001_create_table::Migration,
        )]
    }
}
