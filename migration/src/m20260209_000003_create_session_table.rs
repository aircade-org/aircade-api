use sea_orm_migration::prelude::*;

/// Creates the `session` table for real-time game sessions.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(clippy::enum_variant_names)]
#[derive(DeriveIden)]
enum Session {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    EndedAt,
    HostId,
    GameId,
    GameVersionId,
    SessionCode,
    Status,
    MaxPlayers,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Game {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum GameVersion {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Session::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Session::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Session::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Session::EndedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Session::HostId).uuid().not_null())
                    .col(ColumnDef::new(Session::GameId).uuid().null())
                    .col(ColumnDef::new(Session::GameVersionId).uuid().null())
                    .col(
                        ColumnDef::new(Session::SessionCode)
                            .string_len(10)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Session::Status)
                            .string_len(20)
                            .not_null()
                            .default("lobby"),
                    )
                    .col(
                        ColumnDef::new(Session::MaxPlayers)
                            .integer()
                            .not_null()
                            .default(8),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_host_id")
                            .from(Session::Table, Session::HostId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_game_id")
                            .from(Session::Table, Session::GameId)
                            .to(Game::Table, Game::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_game_version_id")
                            .from(Session::Table, Session::GameVersionId)
                            .to(GameVersion::Table, GameVersion::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await
    }
}
