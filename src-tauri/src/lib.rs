mod app_state;
mod bootstrap;
mod capture_window;
mod commands;
mod dev_fallback;
mod runtime_controller;
mod tray;
mod updater;
mod windowing;

use tauri::Manager;

use app_state::AppState;

pub fn run() {
    let builder = tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState::default());
            bootstrap::bootstrap_runtime(app)?;
            dev_fallback::ensure_dev_server(app)?;
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_app_paths,
            commands::config::get_config,
            commands::config::save_config,
            commands::diagnostics::export_diagnostics_bundle,
            commands::diagnostics::get_diagnostics_overview,
            commands::import_legacy::dry_run_legacy_import_command,
            commands::import_legacy::run_legacy_import_command,
            commands::runtime::get_preview_snapshot,
            commands::runtime::get_runtime_status,
            commands::runtime::restart_runtime,
            commands::runtime::start_runtime,
            commands::runtime::stop_runtime,
            commands::system::get_bootstrap_info,
            commands::system::list_workspace_modules,
            commands::target::list_monitors,
            commands::target::list_target_windows,
            commands::target::locate_target,
            commands::target::pick_target_window,
            commands::target::test_target_capture,
            commands::template::get_template_preview,
            commands::template::import_pasted_template,
            commands::template::import_template,
            commands::template::list_templates,
            commands::template::remove_template,
            commands::template::rename_template,
            commands::updater::check_for_updates,
            commands::updater::get_updater_status,
        ])
        .on_window_event(|window, event| {
            windowing::handle_window_event(window, event);
        });

    builder
        .run(tauri::generate_context!())
        .expect("tauri application failed");
}
