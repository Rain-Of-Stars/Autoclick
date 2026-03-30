use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::StorageError;

const CURRENT_SCHEMA_VERSION: i64 = 1;

pub fn open_database(path: &Path) -> Result<Connection, StorageError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| StorageError::DatabaseInit(err.to_string()))?;
    }

    let connection =
        Connection::open(path).map_err(|err| StorageError::DatabaseInit(err.to_string()))?;
    connection
        .execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            "#,
        )
        .map_err(|err| StorageError::DatabaseInit(err.to_string()))?;
    migrate(&connection)?;
    Ok(connection)
}

pub fn migrate(connection: &Connection) -> Result<(), StorageError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|err| StorageError::Database(err.to_string()))?;

    let current_version = connection
        .query_row("SELECT MAX(version) FROM migrations;", [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .optional()
        .map_err(|err| StorageError::Database(err.to_string()))?
        .flatten()
        .unwrap_or(0);

    if current_version >= CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS config_profiles (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                data TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                hash TEXT NOT NULL,
                source_path TEXT,
                stored_path TEXT,
                width INTEGER NOT NULL DEFAULT 0,
                height INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS template_tags (
                template_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (template_id, tag),
                FOREIGN KEY (template_id) REFERENCES templates(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS run_sessions (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT
            );

            CREATE TABLE IF NOT EXISTS hit_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                template_id TEXT,
                score REAL NOT NULL,
                hit_x INTEGER NOT NULL,
                hit_y INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (run_id) REFERENCES run_sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS config_backups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                data TEXT NOT NULL,
                note TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|err| StorageError::Database(err.to_string()))?;

    connection
        .execute(
            "INSERT OR REPLACE INTO migrations (version, applied_at) VALUES (?1, datetime('now'));",
            [CURRENT_SCHEMA_VERSION],
        )
        .map_err(|err| StorageError::Database(err.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::open_database;

    #[test]
    fn creates_schema_once() {
        let db_path =
            std::env::temp_dir().join(format!("autoclick-storage-{}.db", uuid::Uuid::new_v4()));
        let connection = open_database(&db_path).expect("db should open");
        let table_name: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='config_profiles';",
                [],
                |row| row.get(0),
            )
            .expect("schema should exist");
        assert_eq!(table_name, "config_profiles");
    }
}
