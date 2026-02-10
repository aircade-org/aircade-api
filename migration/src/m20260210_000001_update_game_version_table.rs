use sea_orm_migration::prelude::*;

/// Adds `published_by_id` and renames `change_log` to `changelog` in `game_version`.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add published_by_id column
        manager
            .alter_table(
                Table::alter()
                    .table(GameVersion::Table)
                    .add_column(ColumnDef::new(GameVersion::PublishedById).uuid().null())
                    .to_owned(),
            )
            .await?;

        // Add changelog column (separate from change_log; keep old column for data migration)
        manager
            .alter_table(
                Table::alter()
                    .table(GameVersion::Table)
                    .add_column(ColumnDef::new(GameVersion::Changelog).text().null())
                    .to_owned(),
            )
            .await?;

        // Copy existing change_log data to changelog
        manager
            .exec_stmt(
                sea_orm_migration::prelude::Query::update()
                    .table(GameVersion::Table)
                    .value(GameVersion::Changelog, Expr::col(GameVersion::ChangeLog))
                    .to_owned(),
            )
            .await?;

        // Make game_screen_code and controller_screen_code NOT NULL with default
        // SQLite doesn't support altering column constraints, so we use a workaround
        // For PostgreSQL this would be a direct ALTER COLUMN
        // We'll handle this via application logic instead

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GameVersion::Table)
                    .drop_column(GameVersion::PublishedById)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GameVersion::Table)
                    .drop_column(GameVersion::Changelog)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum GameVersion {
    Table,
    PublishedById,
    Changelog,
    ChangeLog,
}
