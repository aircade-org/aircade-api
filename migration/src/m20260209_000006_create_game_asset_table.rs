use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GameAsset::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameAsset::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GameAsset::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(GameAsset::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(GameAsset::GameId).uuid().not_null())
                    .col(ColumnDef::new(GameAsset::FileName).string().not_null())
                    .col(ColumnDef::new(GameAsset::FileType).string().not_null())
                    .col(ColumnDef::new(GameAsset::FileSize).integer().not_null())
                    .col(ColumnDef::new(GameAsset::FileData).binary().not_null())
                    .col(ColumnDef::new(GameAsset::StorageUrl).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_asset_game_id")
                            .from(GameAsset::Table, GameAsset::GameId)
                            .to(Game::Table, Game::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on game_id for faster asset listing
        manager
            .create_index(
                Index::create()
                    .name("idx_game_asset_game_id")
                    .table(GameAsset::Table)
                    .col(GameAsset::GameId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GameAsset::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GameAsset {
    Table,
    Id,
    CreatedAt,
    DeletedAt,
    GameId,
    FileName,
    FileType,
    FileSize,
    FileData,
    StorageUrl,
}

#[derive(DeriveIden)]
enum Game {
    Table,
    Id,
}
