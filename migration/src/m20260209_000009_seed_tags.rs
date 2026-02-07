use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Convert a UUID string (with dashes) to an `SQLite` hex-blob literal.
///
/// `SeaORM` stores UUID columns as 16-byte BLOBs in `SQLite`, so raw SQL
/// inserts must use `X'...'` notation to match the format.
fn uuid_blob(uuid_str: &str) -> String {
    let hex: String = uuid_str.chars().filter(|c| *c != '-').collect();
    format!("X'{hex}'")
}

/// A single tag definition.
struct Tag {
    id: &'static str,
    name: &'static str,
    slug: &'static str,
    category: &'static str,
}

#[rustfmt::skip]
const TAGS: &[Tag] = &[
    // Genre
    Tag { id: "01000000-0000-4000-8000-000000000001", name: "Racing",       slug: "racing",       category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000002", name: "Trivia",       slug: "trivia",       category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000003", name: "Drawing",      slug: "drawing",      category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000004", name: "Strategy",     slug: "strategy",     category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000005", name: "Puzzle",       slug: "puzzle",       category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000006", name: "Action",       slug: "action",       category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000007", name: "Sports",       slug: "sports",       category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000008", name: "Rhythm",       slug: "rhythm",       category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000009", name: "Party",        slug: "party",        category: "genre" },
    Tag { id: "01000000-0000-4000-8000-000000000010", name: "Word",         slug: "word",         category: "genre" },
    // Mood
    Tag { id: "02000000-0000-4000-8000-000000000001", name: "Competitive",  slug: "competitive",  category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000002", name: "Cooperative",  slug: "cooperative",  category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000003", name: "Relaxed",      slug: "relaxed",      category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000004", name: "Chaotic",      slug: "chaotic",      category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000005", name: "Creative",     slug: "creative",     category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000006", name: "Silly",        slug: "silly",        category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000007", name: "Strategic",    slug: "strategic",    category: "mood" },
    Tag { id: "02000000-0000-4000-8000-000000000008", name: "Fast-Paced",   slug: "fast-paced",   category: "mood" },
    // Player style
    Tag { id: "03000000-0000-4000-8000-000000000001", name: "Free-for-all", slug: "free-for-all", category: "playerStyle" },
    Tag { id: "03000000-0000-4000-8000-000000000002", name: "Teams",        slug: "teams",        category: "playerStyle" },
    Tag { id: "03000000-0000-4000-8000-000000000003", name: "Turn-based",   slug: "turn-based",   category: "playerStyle" },
    Tag { id: "03000000-0000-4000-8000-000000000004", name: "Real-time",    slug: "real-time",    category: "playerStyle" },
    Tag { id: "03000000-0000-4000-8000-000000000005", name: "Solo",         slug: "solo",         category: "playerStyle" },
    Tag { id: "03000000-0000-4000-8000-000000000006", name: "Multiplayer",  slug: "multiplayer",  category: "playerStyle" },
    Tag { id: "03000000-0000-4000-8000-000000000007", name: "Versus",       slug: "versus",       category: "playerStyle" },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();

        for tag in TAGS {
            let sql = if backend == sea_orm::DatabaseBackend::Postgres {
                format!(
                    "INSERT INTO tag (id, name, slug, category) \
                     VALUES ('{id}', '{name}', '{slug}', '{category}') \
                     ON CONFLICT (id) DO NOTHING",
                    id = tag.id,
                    name = tag.name,
                    slug = tag.slug,
                    category = tag.category,
                )
            } else {
                let id_blob = uuid_blob(tag.id);
                format!(
                    "INSERT OR IGNORE INTO tag (id, name, slug, category) \
                     VALUES ({id_blob}, '{name}', '{slug}', '{category}')",
                    id_blob = id_blob,
                    name = tag.name,
                    slug = tag.slug,
                    category = tag.category,
                )
            };
            db.execute(sea_orm::Statement::from_string(backend, sql))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(Query::delete().from_table(TagIden::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TagIden {
    Table,
}
