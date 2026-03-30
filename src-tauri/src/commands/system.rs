use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapInfo {
    pub app_name: &'static str,
    pub version: &'static str,
    pub runtime_path: &'static str,
}

#[tauri::command]
pub fn get_bootstrap_info() -> BootstrapInfo {
    BootstrapInfo {
        app_name: "Autoclick Tauri 2",
        version: env!("CARGO_PKG_VERSION"),
        runtime_path: "rust-workspace",
    }
}

#[tauri::command]
pub fn list_workspace_modules() -> Vec<&'static str> {
    vec![
        "autoclick-domain",
        "autoclick-platform-win",
        "autoclick-storage",
        "autoclick-capture",
        "autoclick-detect",
        "autoclick-input",
        "autoclick-runtime",
        "autoclick-diagnostics",
    ]
}
