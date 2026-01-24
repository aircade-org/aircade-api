use sea_orm_migration::prelude::*;

/// Creates the `user` table with all fields defined in the entity specification.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Email,
    Username,
    DisplayName,
    AvatarUrl,
    Bio,
    EmailVerified,
    Role,
    SubscriptionPlan,
    SubscriptionExpiresAt,
    AccountStatus,
    SuspensionReason,
    LastLoginAt,
    LastLoginIp,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(User::Email)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(User::Username)
                            .string_len(50)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(User::DisplayName).string_len(100).null())
                    .col(ColumnDef::new(User::AvatarUrl).string_len(500).null())
                    .col(ColumnDef::new(User::Bio).string_len(500).null())
                    .col(
                        ColumnDef::new(User::EmailVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(User::Role)
                            .string_len(20)
                            .not_null()
                            .default("user"),
                    )
                    .col(
                        ColumnDef::new(User::SubscriptionPlan)
                            .string_len(20)
                            .not_null()
                            .default("free"),
                    )
                    .col(
                        ColumnDef::new(User::SubscriptionExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(User::AccountStatus)
                            .string_len(20)
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(User::SuspensionReason)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(User::LastLoginAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(User::LastLoginIp).string_len(45).null())
                    .col(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(User::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(User::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}
