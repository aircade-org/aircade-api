pub use sea_orm_migration::prelude::*;

mod m20251218_000001_init;
mod m20260208_000001_create_user_table;
mod m20260208_000002_create_auth_provider_table;
mod m20260208_000003_create_refresh_token_table;
mod m20260209_000001_create_game_table;
mod m20260209_000002_create_game_version_table;
mod m20260209_000003_create_session_table;
mod m20260209_000004_create_player_table;
mod m20260209_000005_seed_pong_game;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251218_000001_init::Migration),
            Box::new(m20260208_000001_create_user_table::Migration),
            Box::new(m20260208_000002_create_auth_provider_table::Migration),
            Box::new(m20260208_000003_create_refresh_token_table::Migration),
            Box::new(m20260209_000001_create_game_table::Migration),
            Box::new(m20260209_000002_create_game_version_table::Migration),
            Box::new(m20260209_000003_create_session_table::Migration),
            Box::new(m20260209_000004_create_player_table::Migration),
            Box::new(m20260209_000005_seed_pong_game::Migration),
        ]
    }
}
