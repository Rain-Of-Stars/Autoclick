use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use autoclick_domain::{config::AppConfig, paths::AppPaths};
use serde::Serialize;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedBundleEntry {
    pub name: String,
    pub source: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub os: String,
    pub arch: String,
    pub current_dir: String,
    pub data_dir: String,
    pub cache_dir: String,
    pub log_dir: String,
    pub templates_dir: String,
    pub debug_dir: String,
    pub db_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub product_name: String,
    pub version: String,
    pub profile: String,
    pub target_os: String,
    pub target_arch: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundleSummary {
    pub archive_path: PathBuf,
    pub entries: Vec<ExportedBundleEntry>,
}

struct PathSanitizer {
    app_root: Option<String>,
    home_dirs: Vec<String>,
}

impl PathSanitizer {
    fn new(paths: &AppPaths) -> Self {
        let app_root = infer_app_root(paths).map(|path| normalize_path(path.as_path()));
        let mut home_dirs = Vec::new();

        for key in ["USERPROFILE", "HOME"] {
            if let Ok(home) = env::var(key) {
                let normalized = home.replace('\\', "/");
                if !normalized.is_empty() && !home_dirs.contains(&normalized) {
                    home_dirs.push(normalized);
                }
            }
        }

        Self {
            app_root,
            home_dirs,
        }
    }

    fn sanitize(&self, path: &Path) -> String {
        let display = normalize_path(path);

        if let Some(app_root) = &self.app_root {
            if let Some(redacted) = replace_path_root(&display, app_root, "<APP_ROOT>") {
                return redacted;
            }
        }

        for home in &self.home_dirs {
            if let Some(redacted) = replace_path_root(&display, home, "<USER_HOME>") {
                return redacted;
            }
        }

        display
    }
}

pub fn export_bundle<R: Serialize>(
    paths: &AppPaths,
    config: &AppConfig,
    runtime: &R,
) -> anyhow::Result<ExportBundleSummary> {
    let sanitizer = PathSanitizer::new(paths);
    let export_dir = paths.cache_dir.join("diagnostics");
    fs::create_dir_all(&export_dir)?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let archive_path = export_dir.join(format!("diagnostics-{stamp}.zip"));
    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut entries = Vec::new();

    write_json_entry(
        &mut zip,
        options,
        "config/config.json",
        config,
        &mut entries,
    )?;
    write_json_entry(
        &mut zip,
        options,
        "runtime/runtime.json",
        runtime,
        &mut entries,
    )?;
    write_json_entry(
        &mut zip,
        options,
        "meta/version.json",
        &VersionInfo {
            product_name: "Autoclick Tauri 2".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            profile: option_env!("PROFILE").unwrap_or("unknown").to_string(),
            target_os: env::consts::OS.to_string(),
            target_arch: env::consts::ARCH.to_string(),
        },
        &mut entries,
    )?;
    write_json_entry(
        &mut zip,
        options,
        "meta/environment.json",
        &build_environment_info(paths, &sanitizer)?,
        &mut entries,
    )?;

    append_directory(
        &mut zip,
        options,
        &paths.log_dir,
        Path::new("logs"),
        &mut entries,
        &sanitizer,
    )?;
    append_directory(
        &mut zip,
        options,
        &paths.debug_dir,
        Path::new("debug"),
        &mut entries,
        &sanitizer,
    )?;

    zip.finish()?;
    Ok(ExportBundleSummary {
        archive_path,
        entries,
    })
}

fn build_environment_info(
    paths: &AppPaths,
    sanitizer: &PathSanitizer,
) -> anyhow::Result<EnvironmentInfo> {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    Ok(EnvironmentInfo {
        os: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        current_dir: sanitizer.sanitize(&current_dir),
        data_dir: sanitizer.sanitize(&paths.data_dir),
        cache_dir: sanitizer.sanitize(&paths.cache_dir),
        log_dir: sanitizer.sanitize(&paths.log_dir),
        templates_dir: sanitizer.sanitize(&paths.templates_dir),
        debug_dir: sanitizer.sanitize(&paths.debug_dir),
        db_path: sanitizer.sanitize(&paths.db_path),
    })
}

fn infer_app_root(paths: &AppPaths) -> Option<PathBuf> {
    common_path_prefix(&[
        paths.data_dir.as_path(),
        paths.cache_dir.as_path(),
        paths.log_dir.as_path(),
        paths.templates_dir.as_path(),
        paths.debug_dir.as_path(),
        paths.db_path.as_path(),
    ])
}

fn common_path_prefix(paths: &[&Path]) -> Option<PathBuf> {
    let mut iter = paths.iter();
    let first = iter.next()?;
    let mut common: Vec<OsString> = first
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();

    for path in iter {
        let components: Vec<OsString> = path
            .components()
            .map(|component| component.as_os_str().to_os_string())
            .collect();
        let shared_len = common
            .iter()
            .zip(components.iter())
            .take_while(|(left, right)| left == right)
            .count();
        common.truncate(shared_len);

        if common.is_empty() {
            return None;
        }
    }

    let mut prefix = PathBuf::new();
    for component in common {
        prefix.push(component);
    }
    Some(prefix)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn replace_path_root(display: &str, root: &str, placeholder: &str) -> Option<String> {
    if display == root {
        return Some(placeholder.to_string());
    }

    let suffix = display.strip_prefix(root)?;
    if suffix.starts_with('/') {
        return Some(format!("{placeholder}{suffix}"));
    }

    None
}

fn write_json_entry<T: Serialize>(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    name: &str,
    value: &T,
    entries: &mut Vec<ExportedBundleEntry>,
) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    zip.start_file(name, options)?;
    zip.write_all(&bytes)?;
    entries.push(ExportedBundleEntry {
        name: name.to_string(),
        source: "json".to_string(),
        size_bytes: bytes.len() as u64,
    });
    Ok(())
}

fn append_directory(
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    source_dir: &Path,
    archive_root: &Path,
    entries: &mut Vec<ExportedBundleEntry>,
    sanitizer: &PathSanitizer,
) -> anyhow::Result<()> {
    if !source_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let child_root = archive_root.join(entry.file_name());
            append_directory(zip, options, &path, &child_root, entries, sanitizer)?;
            continue;
        }

        let bytes = fs::read(&path)?;
        let archive_name = archive_root.join(entry.file_name());
        let archive_name = archive_entry_name(&archive_name);
        zip.start_file(&archive_name, options)?;
        zip.write_all(&bytes)?;
        entries.push(ExportedBundleEntry {
            name: archive_name,
            source: sanitizer.sanitize(&path),
            size_bytes: bytes.len() as u64,
        });
    }

    Ok(())
}

fn archive_entry_name(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::{fs, fs::File, io::Read};

    use autoclick_domain::{config::AppConfig, paths::AppPaths};
    use serde_json::json;
    use uuid::Uuid;
    use zip::ZipArchive;

    use super::export_bundle;

    #[test]
    fn export_bundle_writes_expected_payloads() {
        let root = std::env::temp_dir().join(format!("autoclick-diagnostics-{}", Uuid::new_v4()));
        let paths = AppPaths::from_base_dir(&root);
        for directory in paths.required_directories() {
            fs::create_dir_all(directory).expect("create directory");
        }

        fs::write(paths.log_dir.join("autoclick.log"), "line one").expect("write log");
        fs::write(paths.debug_dir.join("preview.png"), b"png").expect("write debug");

        let summary = export_bundle(&paths, &AppConfig::default(), &json!({ "status": "Idle" }))
            .expect("export bundle");
        assert!(summary.archive_path.exists());
        assert!(
            summary
                .entries
                .iter()
                .any(|entry| entry.name == "config/config.json")
        );
        assert!(
            summary
                .entries
                .iter()
                .any(|entry| entry.name == "logs/autoclick.log")
        );
        assert!(summary.entries.iter().all(|entry| {
            !entry
                .source
                .contains(&root.to_string_lossy().replace('\\', "/"))
        }));

        let file = File::open(&summary.archive_path).expect("open archive");
        let mut archive = ZipArchive::new(file).expect("read archive");
        let mut environment = String::new();
        archive
            .by_name("meta/environment.json")
            .expect("environment entry")
            .read_to_string(&mut environment)
            .expect("read environment");
        assert!(environment.contains("\"os\""));
        assert!(environment.contains("<APP_ROOT>"));
        assert!(!environment.contains(&root.to_string_lossy().replace('\\', "/")));
    }
}
