use sea_orm_migration::prelude::*;

/// Creates the `game` table for storing game metadata.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(clippy::enum_variant_names)]
#[derive(DeriveIden)]
enum Game {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    OwnerId,
    Title,
    Slug,
    Description,
    Thumbnail,
    Technology,
    Status,
    Visibility,
    MinPlayers,
    MaxPlayers,
    PublishedVersionId,
    GameScreenCode,
    ControllerScreenCode,
    PlayCount,
    TotalPlayTime,
    AvgRating,
    ReviewCount,
    ForkedFromId,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[async_trait::async_trait]
#[allow(clippy::too_many_lines)]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Game::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Game::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Game::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Game::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Game::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Game::OwnerId).uuid().not_null())
                    .col(ColumnDef::new(Game::Title).string_len(200).not_null())
                    .col(
                        ColumnDef::new(Game::Slug)
                            .string_len(200)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Game::Description).text().null())
                    .col(ColumnDef::new(Game::Thumbnail).string_len(500).null())
                    .col(
                        ColumnDef::new(Game::Technology)
                            .string_len(20)
                            .not_null()
                            .default("p5js"),
                    )
                    .col(
                        ColumnDef::new(Game::Status)
                            .string_len(20)
                            .not_null()
                            .default("draft"),
                    )
                    .col(
                        ColumnDef::new(Game::Visibility)
                            .string_len(20)
                            .not_null()
                            .default("private"),
                    )
                    .col(
                        ColumnDef::new(Game::MinPlayers)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Game::MaxPlayers)
                            .integer()
                            .not_null()
                            .default(4),
                    )
                    .col(ColumnDef::new(Game::PublishedVersionId).uuid().null())
                    .col(ColumnDef::new(Game::GameScreenCode).text().null())
                    .col(ColumnDef::new(Game::ControllerScreenCode).text().null())
                    .col(
                        ColumnDef::new(Game::PlayCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Game::TotalPlayTime)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Game::AvgRating)
                            .float()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(Game::ReviewCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Game::ForkedFromId).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_owner_id")
                            .from(Game::Table, Game::OwnerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_forked_from_id")
                            .from(Game::Table, Game::ForkedFromId)
                            .to(Game::Table, Game::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Game::Table).to_owned())
            .await
    }
}
