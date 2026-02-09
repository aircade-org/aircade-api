use sea_orm_migration::prelude::*;

/// Creates the `game_version` table with a unique constraint on (`game_id`, `version_number`).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum GameVersion {
    Table,
    Id,
    CreatedAt,
    GameId,
    VersionNumber,
    GameScreenCode,
    ControllerScreenCode,
    ChangeLog,
}

#[derive(DeriveIden)]
enum Game {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GameVersion::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameVersion::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GameVersion::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GameVersion::GameId).uuid().not_null())
                    .col(
                        ColumnDef::new(GameVersion::VersionNumber)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(GameVersion::GameScreenCode).text().null())
                    .col(
                        ColumnDef::new(GameVersion::ControllerScreenCode)
                            .text()
                            .null(),
                    )
                    .col(ColumnDef::new(GameVersion::ChangeLog).text().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_version_game_id")
                            .from(GameVersion::Table, GameVersion::GameId)
                            .to(Game::Table, Game::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint: one version number per game
        manager
            .create_index(
                Index::create()
                    .name("idx_game_version_game_version_number")
                    .table(GameVersion::Table)
                    .col(GameVersion::GameId)
                    .col(GameVersion::VersionNumber)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GameVersion::Table).to_owned())
            .await
    }
}
