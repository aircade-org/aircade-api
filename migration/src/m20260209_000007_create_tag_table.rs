use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tag::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tag::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Tag::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(Tag::Slug).string().not_null().unique_key())
                    .col(ColumnDef::new(Tag::Category).string().not_null())
                    .to_owned(),
            )
            .await?;

        // Add index on category for filtered tag listing
        manager
            .create_index(
                Index::create()
                    .name("idx_tag_category")
                    .table(Tag::Table)
                    .col(Tag::Category)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tag::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tag {
    Table,
    Id,
    Name,
    Slug,
    Category,
}
