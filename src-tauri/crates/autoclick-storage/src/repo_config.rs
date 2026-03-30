use std::path::{Path, PathBuf};

use autoclick_domain::config::AppConfig;
use chrono::Utc;

use crate::{StorageError, migrations::open_database};

pub struct ConfigRepository {
    db_path: PathBuf,
}

impl ConfigRepository {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<Option<AppConfig>, StorageError> {
        let connection = open_database(&self.db_path)?;
        let row = connection.query_row(
            "SELECT data FROM config_profiles WHERE id = 1;",
            [],
            |row| row.get::<_, String>(0),
        );

        match row {
            Ok(payload) => serde_json::from_str(&payload)
                .map(Some)
                .map_err(|err| StorageError::Database(err.to_string())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(StorageError::Database(err.to_string())),
        }
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), StorageError> {
        config
            .validate()
            .map_err(|err| StorageError::Database(err.to_string()))?;
        let connection = open_database(&self.db_path)?;
        let payload =
            serde_json::to_string(config).map_err(|err| StorageError::Database(err.to_string()))?;
        connection
            .execute(
                r#"
                INSERT INTO config_profiles (id, data, updated_at)
                VALUES (1, ?1, ?2)
                ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at;
                "#,
                rusqlite::params![payload, Utc::now().to_rfc3339()],
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        Ok(())
    }

    pub fn backup(&self, note: &str) -> Result<(), StorageError> {
        if let Some(config) = self.load()? {
            let connection = open_database(&self.db_path)?;
            let payload = serde_json::to_string(&config)
                .map_err(|err| StorageError::Database(err.to_string()))?;
            connection
                .execute(
                    "INSERT INTO config_backups (data, note, created_at) VALUES (?1, ?2, ?3);",
                    rusqlite::params![payload, note, Utc::now().to_rfc3339()],
                )
                .map_err(|err| StorageError::Database(err.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use autoclick_domain::config::AppConfig;

    use super::ConfigRepository;

    #[test]
    fn saves_and_loads_config() {
        let db_path =
            std::env::temp_dir().join(format!("autoclick-config-{}.db", uuid::Uuid::new_v4()));
        let repository = ConfigRepository::new(&db_path);
        let config = AppConfig::default();
        repository.save(&config).expect("save should succeed");
        let loaded = repository
            .load()
            .expect("load should succeed")
            .expect("config should exist");
        assert_eq!(loaded.capture.target_fps, config.capture.target_fps);
    }
}
