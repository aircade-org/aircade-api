use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GameTag::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(GameTag::GameId).uuid().not_null())
                    .col(ColumnDef::new(GameTag::TagId).uuid().not_null())
                    .primary_key(Index::create().col(GameTag::GameId).col(GameTag::TagId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_tag_game_id")
                            .from(GameTag::Table, GameTag::GameId)
                            .to(Game::Table, Game::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_tag_tag_id")
                            .from(GameTag::Table, GameTag::TagId)
                            .to(Tag::Table, Tag::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on tag_id for reverse lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_game_tag_tag_id")
                    .table(GameTag::Table)
                    .col(GameTag::TagId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GameTag::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GameTag {
    Table,
    GameId,
    TagId,
}

#[derive(DeriveIden)]
enum Game {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tag {
    Table,
    Id,
}
