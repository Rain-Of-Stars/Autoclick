use std::path::{Path, PathBuf};

use autoclick_domain::{config::AppConfig, migration::migrate_legacy_value};
use serde::{Deserialize, Serialize};

use crate::{
    StorageError, migrations::open_database, repo_config::ConfigRepository,
    repo_template::TemplateRepository, template_fs::import_template_file,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportReport {
    pub config_imported: bool,
    pub templates_imported: usize,
    pub warnings: Vec<String>,
}

pub fn dry_run_import(
    app_paths: &autoclick_domain::paths::AppPaths,
    legacy_root: impl AsRef<Path>,
) -> Result<LegacyImportReport, StorageError> {
    let legacy_root = legacy_root.as_ref();
    let mut warnings = Vec::new();
    if !legacy_root.join("config.json").exists() && !legacy_root.join("app.db").exists() {
        warnings.push("未找到旧版 config.json 或 app.db。".to_string());
    }
    if !app_paths.templates_dir.exists() {
        warnings.push("新模板目录将在导入时创建。".to_string());
    }
    Ok(LegacyImportReport {
        config_imported: false,
        templates_imported: 0,
        warnings,
    })
}

pub fn import_legacy(
    app_paths: &autoclick_domain::paths::AppPaths,
    legacy_root: impl AsRef<Path>,
) -> Result<LegacyImportReport, StorageError> {
    let legacy_root = legacy_root.as_ref();
    let config = load_legacy_config(legacy_root)?;
    let config_repository = ConfigRepository::new(&app_paths.db_path);
    let template_repository = TemplateRepository::new(&app_paths.db_path);
    config_repository.save(&config)?;
    config_repository.backup("legacy-import")?;

    let mut imported_templates = 0usize;
    let mut warnings = Vec::new();
    for template in &config.templates {
        let source_path = template.source_path.clone().unwrap_or_default();
        let Some(path) = resolve_legacy_template_path(legacy_root, &source_path) else {
            warnings.push(format!("模板源文件不存在: {}", source_path));
            continue;
        };
        let imported = import_template_file(app_paths, path, &template.tags)?;
        template_repository.upsert(&imported)?;
        imported_templates += 1;
    }

    Ok(LegacyImportReport {
        config_imported: true,
        templates_imported: imported_templates,
        warnings,
    })
}

fn load_legacy_config(legacy_root: &Path) -> Result<AppConfig, StorageError> {
    if legacy_root.join("config.json").exists() {
        let payload = std::fs::read_to_string(legacy_root.join("config.json"))
            .map_err(|err| StorageError::LegacyImport(err.to_string()))?;
        let value: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|err| StorageError::LegacyImport(err.to_string()))?;
        return migrate_legacy_value(&value)
            .map_err(|err| StorageError::LegacyImport(err.to_string()));
    }

    let legacy_db = legacy_root.join("app.db");
    let connection = open_database(&legacy_db)?;
    let payload: String = connection
        .query_row("SELECT data FROM config WHERE id = 1;", [], |row| {
            row.get(0)
        })
        .map_err(|err| StorageError::LegacyImport(err.to_string()))?;
    let value: serde_json::Value = serde_json::from_str(&payload)
        .map_err(|err| StorageError::LegacyImport(err.to_string()))?;
    migrate_legacy_value(&value).map_err(|err| StorageError::LegacyImport(err.to_string()))
}

fn resolve_legacy_template_path(legacy_root: &Path, value: &str) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    let direct = PathBuf::from(value);
    if direct.exists() {
        return Some(direct);
    }
    let stripped = value.strip_prefix("db://template/").unwrap_or(value);
    let candidate = legacy_root.join("storage").join("images").join(stripped);
    if candidate.exists() {
        return Some(candidate);
    }
    None
}

#[cfg(test)]
mod tests {
    use autoclick_domain::paths::AppPaths;

    use super::{dry_run_import, import_legacy};

    #[test]
    fn dry_run_warns_when_legacy_files_missing() {
        let base_dir =
            std::env::temp_dir().join(format!("autoclick-legacy-dry-run-{}", uuid::Uuid::new_v4()));
        let app_paths = AppPaths::from_base_dir(base_dir.join("app"));
        let report = dry_run_import(&app_paths, &base_dir).expect("dry run");
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn imports_legacy_json_config() {
        let root =
            std::env::temp_dir().join(format!("autoclick-legacy-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("legacy root");
        std::fs::write(
            root.join("config.json"),
            r#"{"template_path":"","template_paths":[],"threshold":0.9,"fps_max":24,"click_method":"message"}"#,
        )
        .expect("legacy config");
        let app_paths = AppPaths::from_base_dir(root.join("new-app"));
        let report = import_legacy(&app_paths, &root).expect("import should succeed");
        assert!(report.config_imported);
    }
}
