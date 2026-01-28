use sea_orm_migration::prelude::*;

/// Seeds the database with a Pong game record and corresponding `GameVersion`
/// for the proof-of-concept milestone. The actual `p5.js` code lives in the
/// frontend; `game_screen_code` / `controller_screen_code` contain placeholders.
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

/// Build an upsert statement for the system user, adapted per backend.
fn user_sql(backend: sea_orm::DatabaseBackend, id: &str) -> String {
    if backend == sea_orm::DatabaseBackend::Postgres {
        format!(
            "INSERT INTO \"user\" (id, email, username, email_verified, role, \
             subscription_plan, account_status, created_at, updated_at) \
             VALUES ('{id}', 'system@aircade.dev', 'aircade-system', \
             true, 'admin', 'free', 'active', NOW(), NOW()) \
             ON CONFLICT (id) DO NOTHING"
        )
    } else {
        let id_blob = uuid_blob(id);
        format!(
            "INSERT OR IGNORE INTO \"user\" (id, email, username, email_verified, role, \
             subscription_plan, account_status, created_at, updated_at) \
             VALUES ({id_blob}, 'system@aircade.dev', 'aircade-system', \
             1, 'admin', 'free', 'active', \
             '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')"
        )
    }
}

/// Build an upsert statement for the Pong game.
fn game_sql(backend: sea_orm::DatabaseBackend, game_id: &str, owner_id: &str) -> String {
    if backend == sea_orm::DatabaseBackend::Postgres {
        format!(
            "INSERT INTO game (id, created_at, updated_at, owner_id, title, slug, \
             description, technology, status, visibility, min_players, max_players, \
             game_screen_code, controller_screen_code, play_count, total_play_time, \
             avg_rating, review_count) \
             VALUES ('{game_id}', NOW(), NOW(), '{owner_id}', 'Pong', 'pong', \
             'Classic single-player Pong. Control the paddle from your phone!', \
             'p5js', 'published', 'public', 1, 1, \
             '// Game screen code loaded from frontend', \
             '// Controller screen code loaded from frontend', \
             0, 0, 0.0, 0) \
             ON CONFLICT (id) DO NOTHING"
        )
    } else {
        let gid = uuid_blob(game_id);
        let oid = uuid_blob(owner_id);
        format!(
            "INSERT OR IGNORE INTO game (id, created_at, updated_at, owner_id, title, \
             slug, description, technology, status, visibility, min_players, max_players, \
             game_screen_code, controller_screen_code, play_count, total_play_time, \
             avg_rating, review_count) \
             VALUES ({gid}, '2026-01-01T00:00:00+00:00', \
             '2026-01-01T00:00:00+00:00', \
             {oid}, 'Pong', 'pong', \
             'Classic single-player Pong. Control the paddle from your phone!', \
             'p5js', 'published', 'public', 1, 1, \
             '// Game screen code loaded from frontend', \
             '// Controller screen code loaded from frontend', \
             0, 0, 0.0, 0)"
        )
    }
}

/// Build an upsert statement for the Pong game version.
fn version_sql(backend: sea_orm::DatabaseBackend, ver_id: &str, game_id: &str) -> String {
    if backend == sea_orm::DatabaseBackend::Postgres {
        format!(
            "INSERT INTO game_version (id, created_at, game_id, version_number, \
             game_screen_code, controller_screen_code, change_log) \
             VALUES ('{ver_id}', NOW(), '{game_id}', 1, \
             '// Game screen code loaded from frontend', \
             '// Controller screen code loaded from frontend', \
             'Initial Pong PoC release') \
             ON CONFLICT (id) DO NOTHING"
        )
    } else {
        let vid = uuid_blob(ver_id);
        let gid = uuid_blob(game_id);
        format!(
            "INSERT OR IGNORE INTO game_version (id, created_at, game_id, \
             version_number, game_screen_code, controller_screen_code, change_log) \
             VALUES ({vid}, '2026-01-01T00:00:00+00:00', {gid}, 1, \
             '// Game screen code loaded from frontend', \
             '// Controller screen code loaded from frontend', \
             'Initial Pong PoC release')"
        )
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let system_user_id = "00000000-0000-0000-0000-000000000001";
        let pong_game_id = "00000000-0000-0000-0000-000000000010";
        let pong_version_id = "00000000-0000-0000-0000-000000000011";

        let backend = manager.get_database_backend();
        let conn = manager.get_connection();

        conn.execute_unprepared(&user_sql(backend, system_user_id))
            .await?;
        conn.execute_unprepared(&game_sql(backend, pong_game_id, system_user_id))
            .await?;
        conn.execute_unprepared(&version_sql(backend, pong_version_id, pong_game_id))
            .await?;

        // Link the published version back to the game
        let update = if backend == sea_orm::DatabaseBackend::Postgres {
            format!(
                "UPDATE game SET published_version_id = '{pong_version_id}' \
                 WHERE id = '{pong_game_id}'"
            )
        } else {
            let vid = uuid_blob(pong_version_id);
            let gid = uuid_blob(pong_game_id);
            format!("UPDATE game SET published_version_id = {vid} WHERE id = {gid}")
        };
        conn.execute_unprepared(&update).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let pong_game_id = "00000000-0000-0000-0000-000000000010";
        let pong_version_id = "00000000-0000-0000-0000-000000000011";
        let system_user_id = "00000000-0000-0000-0000-000000000001";

        let backend = manager.get_database_backend();
        let conn = manager.get_connection();

        let (ver_del, game_del, user_del) = if backend == sea_orm::DatabaseBackend::Postgres {
            (
                format!("DELETE FROM game_version WHERE id = '{pong_version_id}'"),
                format!("DELETE FROM game WHERE id = '{pong_game_id}'"),
                format!("DELETE FROM \"user\" WHERE id = '{system_user_id}'"),
            )
        } else {
            (
                format!(
                    "DELETE FROM game_version WHERE id = {}",
                    uuid_blob(pong_version_id)
                ),
                format!("DELETE FROM game WHERE id = {}", uuid_blob(pong_game_id)),
                format!(
                    "DELETE FROM \"user\" WHERE id = {}",
                    uuid_blob(system_user_id)
                ),
            )
        };

        conn.execute_unprepared(&ver_del).await?;
        conn.execute_unprepared(&game_del).await?;
        conn.execute_unprepared(&user_del).await?;

        Ok(())
    }
}
