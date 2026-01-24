use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Games::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Games::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Games::Code)
                            .string()
                            .string_len(6)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Games::HostId).integer().not_null())
                    .col(
                        ColumnDef::new(Games::Status)
                            .string()
                            .not_null()
                            .default("lobby"),
                    )
                    .col(ColumnDef::new(Games::Settings).json())
                    .col(
                        ColumnDef::new(Games::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_games_host_id")
                            .from(Games::Table, Games::HostId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on code for faster game lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_games_code")
                    .table(Games::Table)
                    .col(Games::Code)
                    .to_owned(),
            )
            .await?;

        // Create index on status for filtering active games
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_games_status")
                    .table(Games::Table)
                    .col(Games::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Games::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Games {
    Table,
    Id,
    Code,
    HostId,
    Status,
    Settings,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
