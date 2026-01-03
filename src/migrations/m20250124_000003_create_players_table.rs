use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Players::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Players::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Players::GameId).integer().not_null())
                    .col(ColumnDef::new(Players::UserId).integer().not_null())
                    .col(ColumnDef::new(Players::Nickname).string().not_null())
                    .col(ColumnDef::new(Players::Color).string().not_null())
                    .col(
                        ColumnDef::new(Players::JoinedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_players_game_id")
                            .from(Players::Table, Players::GameId)
                            .to(Games::Table, Games::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_players_user_id")
                            .from(Players::Table, Players::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on game_id for faster player lookups per game
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_players_game_id")
                    .table(Players::Table)
                    .col(Players::GameId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for game_id + user_id (one user can only join a game once)
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_players_game_user_unique")
                    .table(Players::Table)
                    .col(Players::GameId)
                    .col(Players::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for game_id + nickname (nicknames must be unique per game)
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_players_game_nickname_unique")
                    .table(Players::Table)
                    .col(Players::GameId)
                    .col(Players::Nickname)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Players::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Players {
    Table,
    Id,
    GameId,
    UserId,
    Nickname,
    Color,
    JoinedAt,
}

#[derive(DeriveIden)]
enum Games {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
