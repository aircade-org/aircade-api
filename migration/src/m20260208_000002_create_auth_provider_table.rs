use sea_orm_migration::prelude::*;

/// Creates the `auth_provider` table with foreign key to `user` and composite unique constraint.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum AuthProvider {
    Table,
    Id,
    UserId,
    Provider,
    ProviderId,
    PasswordHash,
    ProviderEmail,
    VerificationToken,
    TokenExpiresAt,
    CreatedAt,
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
                    .table(AuthProvider::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuthProvider::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuthProvider::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(AuthProvider::Provider)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuthProvider::ProviderId)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AuthProvider::PasswordHash)
                            .string_len(255)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuthProvider::ProviderEmail)
                            .string_len(255)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuthProvider::VerificationToken)
                            .string_len(255)
                            .null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AuthProvider::TokenExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(AuthProvider::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_auth_provider_user_id")
                            .from(AuthProvider::Table, AuthProvider::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint: one provider type per user
        manager
            .create_index(
                Index::create()
                    .name("idx_auth_provider_user_provider")
                    .table(AuthProvider::Table)
                    .col(AuthProvider::UserId)
                    .col(AuthProvider::Provider)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthProvider::Table).to_owned())
            .await
    }
}
