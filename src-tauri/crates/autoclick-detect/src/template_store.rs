use std::{collections::HashMap, sync::Arc};

use autoclick_domain::template::TemplateRef;
use autoclick_storage::repo_template::TemplateRepository;
use image::GrayImage;
use parking_lot::RwLock;

use crate::{DetectError, preprocess::load_gray_image};

#[derive(Debug, Clone)]
pub struct LoadedTemplate {
    pub meta: TemplateRef,
    pub image: GrayImage,
}

#[derive(Debug, Default)]
pub struct TemplateStore {
    cache: RwLock<HashMap<String, Arc<LoadedTemplate>>>,
}

impl TemplateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&self, template: &TemplateRef) -> Result<Arc<LoadedTemplate>, DetectError> {
        if let Some(cached) = self.cache.read().get(&template.hash).cloned() {
            return Ok(cached);
        }

        let path = template
            .stored_path
            .as_ref()
            .or(template.source_path.as_ref())
            .ok_or_else(|| DetectError::Image("模板缺少可读取路径".to_string()))?;
        let image = load_gray_image(path)?;
        let loaded = Arc::new(LoadedTemplate {
            meta: template.clone(),
            image,
        });
        self.cache
            .write()
            .insert(template.hash.clone(), loaded.clone());
        Ok(loaded)
    }

    pub fn load_all(
        &self,
        templates: &[TemplateRef],
    ) -> Result<Vec<Arc<LoadedTemplate>>, DetectError> {
        templates
            .iter()
            .map(|template| self.load(template))
            .collect()
    }

    pub fn load_from_repository(
        &self,
        repository: &TemplateRepository,
    ) -> Result<Vec<Arc<LoadedTemplate>>, DetectError> {
        let templates = repository
            .list()
            .map_err(|err| DetectError::Image(err.to_string()))?;
        self.load_all(&templates)
    }

    pub fn invalidate(&self, hash: &str) {
        self.cache.write().remove(hash);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use autoclick_domain::template::TemplateRef;
    use autoclick_storage::repo_template::TemplateRepository;

    use super::TemplateStore;

    #[test]
    fn template_store_returns_cached_entry() {
        let dir = std::env::temp_dir().join(format!(
            "autoclick-detect-template-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("template.png");
        image::GrayImage::from_pixel(2, 2, image::Luma([128]))
            .save(&path)
            .expect("save");
        let mut template = TemplateRef::new("sample");
        template.hash = "hash-1".to_string();
        template.stored_path = Some(path.to_string_lossy().to_string());
        let store = TemplateStore::new();
        let first = store.load(&template).expect("first load");
        let second = store.load(&template).expect("second load");
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn template_store_loads_from_repository() {
        let dir =
            std::env::temp_dir().join(format!("autoclick-detect-repo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("template.png");
        image::GrayImage::from_pixel(3, 3, image::Luma([200]))
            .save(&path)
            .expect("save");

        let db_path = dir.join("templates.db");
        let repository = TemplateRepository::new(&db_path);
        let mut template = TemplateRef::new("repo-sample");
        template.hash = "repo-hash".to_string();
        template.stored_path = Some(path.to_string_lossy().to_string());
        repository.upsert(&template).expect("upsert");

        let store = TemplateStore::new();
        let templates = store
            .load_from_repository(&repository)
            .expect("load repository");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].meta.name, "repo-sample");
    }

    #[test]
    fn invalidate_forces_store_to_reload_template_metadata() {
        let dir = std::env::temp_dir().join(format!(
            "autoclick-detect-template-invalidate-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("template.png");
        image::GrayImage::from_pixel(2, 2, image::Luma([128]))
            .save(&path)
            .expect("save");

        let mut first_template = TemplateRef::new("first-name");
        first_template.hash = "shared-hash".to_string();
        first_template.stored_path = Some(path.to_string_lossy().to_string());

        let store = TemplateStore::new();
        let first = store.load(&first_template).expect("first load");

        store.invalidate(&first_template.hash);

        let mut second_template = first_template.clone();
        second_template.name = "second-name".to_string();
        let second = store.load(&second_template).expect("second load");

        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!(second.meta.name, "second-name");
    }
}
