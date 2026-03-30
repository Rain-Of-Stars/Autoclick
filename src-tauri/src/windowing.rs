use tauri::{Manager, Window, WindowEvent};

use crate::tray;

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" {
            api.prevent_close();
            let _ = window.hide();
            let _ = tray::sync_tray(window.app_handle());
        }
    }
}
