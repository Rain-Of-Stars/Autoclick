use std::path::{Path, PathBuf};

use autoclick_domain::{paths::AppPaths, template::TemplateRef};
use sha2::{Digest, Sha256};

use crate::StorageError;

pub fn import_template_file(
    paths: &AppPaths,
    file_path: impl AsRef<Path>,
    tags: &[String],
) -> Result<TemplateRef, StorageError> {
    let file_path = file_path.as_ref();
    let bytes =
        std::fs::read(file_path).map_err(|err| StorageError::TemplateFs(err.to_string()))?;
    let name = file_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("template")
        .to_string();
    import_template_bytes(
        paths,
        &bytes,
        name,
        Some(file_path.to_string_lossy().to_string()),
        tags,
    )
}

pub fn import_template_bytes(
    paths: &AppPaths,
    bytes: &[u8],
    name: impl Into<String>,
    source_path: Option<String>,
    tags: &[String],
) -> Result<TemplateRef, StorageError> {
    let hash = format!("{:x}", Sha256::digest(bytes));
    let format =
        image::guess_format(bytes).map_err(|err| StorageError::TemplateFs(err.to_string()))?;
    let extension = match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::Bmp => "bmp",
        image::ImageFormat::WebP => "webp",
        _ => "bin",
    };
    let stored_path = paths.templates_dir.join(format!("{}.{}", hash, extension));
    if !stored_path.exists() {
        std::fs::create_dir_all(&paths.templates_dir)
            .map_err(|err| StorageError::TemplateFs(err.to_string()))?;
        std::fs::write(&stored_path, bytes)
            .map_err(|err| StorageError::TemplateFs(err.to_string()))?;
    }
    let image =
        image::load_from_memory(bytes).map_err(|err| StorageError::TemplateFs(err.to_string()))?;
    let mut template = TemplateRef::new(name.into());
    template.hash = hash;
    template.source_path = source_path;
    template.stored_path = Some(stored_path.to_string_lossy().to_string());
    template.width = image.width();
    template.height = image.height();
    template.tags = tags.to_vec();
    Ok(template)
}

pub fn remove_managed_template_file(
    paths: &AppPaths,
    stored_path: impl AsRef<Path>,
) -> Result<(), StorageError> {
    let stored_path = stored_path.as_ref();
    let managed_root = canonicalize_lossy(&paths.templates_dir);
    let candidate = canonicalize_lossy(stored_path);
    if candidate.starts_with(&managed_root) && stored_path.exists() {
        std::fs::remove_file(stored_path)
            .map_err(|err| StorageError::TemplateFs(err.to_string()))?;
    }
    Ok(())
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use autoclick_domain::paths::AppPaths;

    use super::{import_template_bytes, import_template_file};

    #[test]
    fn imports_template_and_keeps_hash_stable() {
        let base_dir =
            std::env::temp_dir().join(format!("autoclick-template-fs-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::from_base_dir(&base_dir);
        std::fs::create_dir_all(&paths.templates_dir).expect("template dir");
        let source = base_dir.join("source.png");
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        image.save(&source).expect("source image");
        let template = import_template_file(&paths, &source, &["red".to_string()]).expect("import");
        assert!(
            template
                .stored_path
                .unwrap_or_default()
                .contains("templates")
        );
        assert_eq!(template.width, 2);
    }

    #[test]
    fn imports_template_from_memory_bytes() {
        let base_dir =
            std::env::temp_dir().join(format!("autoclick-template-bytes-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::from_base_dir(&base_dir);
        std::fs::create_dir_all(&paths.templates_dir).expect("template dir");
        let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([0, 255, 0, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("png bytes");

        let template = import_template_bytes(
            &paths,
            &bytes,
            "captured-template",
            Some("capture://window/100".to_string()),
            &["capture".to_string()],
        )
        .expect("import");

        assert_eq!(template.name, "captured-template");
        assert_eq!(template.width, 3);
        assert_eq!(template.height, 2);
        assert_eq!(
            template.source_path.as_deref(),
            Some("capture://window/100")
        );
    }
}
