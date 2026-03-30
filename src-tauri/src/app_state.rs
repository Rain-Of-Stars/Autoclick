use autoclick_domain::{config::AppConfig, paths::AppPaths, template::TemplateRef};
use autoclick_storage::{repo_config::ConfigRepository, repo_template::TemplateRepository};
use parking_lot::RwLock;

use crate::runtime_controller::RuntimeController;

#[derive(Default)]
pub struct AppState {
    pub paths: RwLock<Option<AppPaths>>,
    pub runtime: RuntimeController,
}

impl AppState {
    pub fn set_paths(&self, paths: AppPaths) {
        self.paths.write().replace(paths);
    }

    pub fn app_paths(&self) -> Result<AppPaths, String> {
        self.paths
            .read()
            .clone()
            .ok_or_else(|| "应用路径尚未初始化".to_string())
    }

    pub fn config_repository(&self) -> Result<ConfigRepository, String> {
        Ok(ConfigRepository::new(self.app_paths()?.db_path))
    }

    pub fn template_repository(&self) -> Result<TemplateRepository, String> {
        Ok(TemplateRepository::new(self.app_paths()?.db_path))
    }

    pub fn load_or_default_config(&self) -> Result<AppConfig, String> {
        let repository = self.config_repository()?;
        match repository.load().map_err(|err| err.to_string())? {
            Some(config) => Ok(config),
            None => {
                let config = AppConfig::default();
                repository.save(&config).map_err(|err| err.to_string())?;
                Ok(config)
            }
        }
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let repository = self.config_repository()?;
        repository.save(config).map_err(|err| err.to_string())
    }

    pub fn list_templates(&self) -> Result<Vec<TemplateRef>, String> {
        let repository = self.template_repository()?;
        repository.list().map_err(|err| err.to_string())
    }

    pub fn sync_templates_into_config(&self) -> Result<Vec<TemplateRef>, String> {
        let templates = self.list_templates()?;
        let mut config = self.load_or_default_config()?;
        config.templates = templates.clone();
        self.save_config(&config)?;
        Ok(templates)
    }
}
