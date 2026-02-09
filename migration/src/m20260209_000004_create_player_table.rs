use sea_orm_migration::prelude::*;

/// Creates the `player` table for tracking session participants.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Player {
    Table,
    Id,
    CreatedAt,
    SessionId,
    UserId,
    DisplayName,
    AvatarUrl,
    ConnectionStatus,
    LeftAt,
}

#[derive(DeriveIden)]
enum Session {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Player::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Player::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Player::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Player::SessionId).uuid().not_null())
                    .col(ColumnDef::new(Player::UserId).uuid().null())
                    .col(
                        ColumnDef::new(Player::DisplayName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Player::AvatarUrl).string_len(500).null())
                    .col(
                        ColumnDef::new(Player::ConnectionStatus)
                            .string_len(20)
                            .not_null()
                            .default("connected"),
                    )
                    .col(
                        ColumnDef::new(Player::LeftAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_session_id")
                            .from(Player::Table, Player::SessionId)
                            .to(Session::Table, Session::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_user_id")
                            .from(Player::Table, Player::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Player::Table).to_owned())
            .await
    }
}
