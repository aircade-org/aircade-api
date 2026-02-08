pub use sea_orm_migration::prelude::*;

mod m20251218_000001_init;
mod m20260208_000001_create_user_table;
mod m20260208_000002_create_auth_provider_table;
mod m20260208_000003_create_refresh_token_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251218_000001_init::Migration),
            Box::new(m20260208_000001_create_user_table::Migration),
            Box::new(m20260208_000002_create_auth_provider_table::Migration),
            Box::new(m20260208_000003_create_refresh_token_table::Migration),
        ]
    }
}
