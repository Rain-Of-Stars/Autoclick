use std::path::{Path, PathBuf};

use autoclick_domain::template::TemplateRef;

use crate::{StorageError, migrations::open_database};

pub struct TemplateRepository {
    db_path: PathBuf,
}

impl TemplateRepository {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    pub fn upsert(&self, template: &TemplateRef) -> Result<(), StorageError> {
        let connection = open_database(&self.db_path)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|err| StorageError::Database(err.to_string()))?;
        transaction
            .execute(
                r#"
                INSERT INTO templates (id, name, hash, source_path, stored_path, width, height, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    hash = excluded.hash,
                    source_path = excluded.source_path,
                    stored_path = excluded.stored_path,
                    width = excluded.width,
                    height = excluded.height;
                "#,
                rusqlite::params![
                    template.id.to_string(),
                    &template.name,
                    &template.hash,
                    &template.source_path,
                    &template.stored_path,
                    template.width,
                    template.height,
                    template.created_at.to_rfc3339(),
                ],
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        transaction
            .execute(
                "DELETE FROM template_tags WHERE template_id = ?1;",
                [template.id.to_string()],
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        for tag in &template.tags {
            transaction
                .execute(
                    "INSERT INTO template_tags (template_id, tag) VALUES (?1, ?2);",
                    rusqlite::params![template.id.to_string(), tag],
                )
                .map_err(|err| StorageError::Database(err.to_string()))?;
        }
        transaction
            .commit()
            .map_err(|err| StorageError::Database(err.to_string()))?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<TemplateRef>, StorageError> {
        let connection = open_database(&self.db_path)?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, hash, source_path, stored_path, width, height, created_at FROM templates ORDER BY created_at DESC;",
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let created_at: String = row.get(7)?;
                Ok(TemplateRef {
                    id: uuid::Uuid::parse_str(&id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
                    name: row.get(1)?,
                    hash: row.get(2)?,
                    source_path: row.get(3)?,
                    stored_path: row.get(4)?,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    tags: Vec::new(),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|value| value.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                })
            })
            .map_err(|err| StorageError::Database(err.to_string()))?;

        let mut templates = Vec::new();
        for row in rows {
            let mut template = row.map_err(|err| StorageError::Database(err.to_string()))?;
            let mut tag_statement = connection
                .prepare("SELECT tag FROM template_tags WHERE template_id = ?1 ORDER BY tag;")
                .map_err(|err| StorageError::Database(err.to_string()))?;
            let tags = tag_statement
                .query_map([template.id.to_string()], |tag_row| {
                    tag_row.get::<_, String>(0)
                })
                .map_err(|err| StorageError::Database(err.to_string()))?;
            template.tags = tags
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| StorageError::Database(err.to_string()))?;
            templates.push(template);
        }
        Ok(templates)
    }

    pub fn delete(&self, template_id: uuid::Uuid) -> Result<(), StorageError> {
        let connection = open_database(&self.db_path)?;
        connection
            .execute(
                "DELETE FROM templates WHERE id = ?1;",
                [template_id.to_string()],
            )
            .map_err(|err| StorageError::Database(err.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use autoclick_domain::template::TemplateRef;

    use super::TemplateRepository;

    #[test]
    fn persists_template_metadata() {
        let db_path =
            std::env::temp_dir().join(format!("autoclick-template-{}.db", uuid::Uuid::new_v4()));
        let repository = TemplateRepository::new(&db_path);
        let mut template = TemplateRef::new("sample");
        template.hash = "hash".to_string();
        template.tags = vec!["ui".to_string(), "smoke".to_string()];
        repository.upsert(&template).expect("upsert should work");
        let templates = repository.list().expect("list should work");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].tags.len(), 2);
    }
}
