use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::{StorageError, migrations::open_database};

pub struct RunRepository {
    db_path: PathBuf,
}

impl RunRepository {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    pub fn start_run(&self, status: &str) -> Result<Uuid, StorageError> {
        let run_id = Uuid::new_v4();
        let connection = open_database(&self.db_path)?;
        connection
            .execute(
                "INSERT INTO run_sessions (id, status, started_at) VALUES (?1, ?2, ?3);",
                rusqlite::params![run_id.to_string(), status, Utc::now().to_rfc3339()],
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        Ok(run_id)
    }

    pub fn finish_run(&self, run_id: Uuid, status: &str) -> Result<(), StorageError> {
        let connection = open_database(&self.db_path)?;
        connection
            .execute(
                "UPDATE run_sessions SET status = ?1, ended_at = ?2 WHERE id = ?3;",
                rusqlite::params![status, Utc::now().to_rfc3339(), run_id.to_string()],
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        Ok(())
    }
}
