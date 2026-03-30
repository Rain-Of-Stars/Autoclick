use anyhow::Result;
use autoclick_domain::types::RuntimeStatus;
use tauri::{
    AppHandle, Manager, Runtime,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{app_state::AppState, commands::runtime::load_runtime_launch_context};

const TRAY_ID: &str = "main";
const MENU_STATUS: &str = "tray-status";
const MENU_WINDOW_STATE: &str = "tray-window-state";
const MENU_SHOW: &str = "tray-show";
const MENU_HIDE: &str = "tray-hide";
const MENU_START: &str = "tray-start";
const MENU_RESTART: &str = "tray-restart";
const MENU_STOP: &str = "tray-stop";
const MENU_EXIT: &str = "tray-exit";

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let menu = build_tray_menu(app)?;

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(build_tooltip(app)))?;
        tray.set_show_menu_on_left_click(false)?;
        tray.on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()));
        tray.on_tray_icon_event(|tray, event| handle_tray_icon_event(tray.app_handle(), &event));
        return Ok(());
    }

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(build_tooltip(app))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| handle_tray_icon_event(tray.app_handle(), &event))
        .build(app)?;
    Ok(())
}

pub fn sync_tray<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_tray_menu(app)?))?;
        tray.set_tooltip(Some(build_tooltip(app)))?;
        tray.set_show_menu_on_left_click(false)?;
    }
    Ok(())
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>> {
    let snapshot = app
        .try_state::<AppState>()
        .map(|state| state.runtime.snapshot())
        .unwrap_or_default();
    let window_state = read_window_state(app);

    let status = MenuItem::with_id(
        app,
        MENU_STATUS,
        format!("状态: {}", runtime_status_text(snapshot.status)),
        false,
        None::<&str>,
    )?;
    let window = MenuItem::with_id(
        app,
        MENU_WINDOW_STATE,
        format!("主界面: {}", window_state_text(&window_state)),
        false,
        None::<&str>,
    )?;
    let show = MenuItem::with_id(
        app,
        MENU_SHOW,
        if window_state.show_enabled {
            "显示主界面"
        } else {
            "主界面已在前台"
        },
        window_state.show_enabled,
        None::<&str>,
    )?;
    let hide = MenuItem::with_id(
        app,
        MENU_HIDE,
        if window_state.hide_enabled {
            "隐藏到系统托盘"
        } else {
            "主界面已隐藏"
        },
        window_state.hide_enabled,
        None::<&str>,
    )?;
    let start = MenuItem::with_id(
        app,
        MENU_START,
        "开始扫描",
        matches!(
            snapshot.status,
            RuntimeStatus::Idle | RuntimeStatus::Faulted
        ),
        None::<&str>,
    )?;
    let restart = MenuItem::with_id(
        app,
        MENU_RESTART,
        "重启链路",
        matches!(
            snapshot.status,
            RuntimeStatus::Running | RuntimeStatus::CoolingDown | RuntimeStatus::Recovering
        ),
        None::<&str>,
    )?;
    let stop = MenuItem::with_id(
        app,
        MENU_STOP,
        "停止扫描",
        !matches!(
            snapshot.status,
            RuntimeStatus::Idle | RuntimeStatus::Faulted | RuntimeStatus::Stopping
        ),
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let exit = MenuItem::with_id(app, MENU_EXIT, "退出程序", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &status,
            &window,
            &separator,
            &show,
            &hide,
            &start,
            &restart,
            &stop,
            &separator2,
            &exit,
        ],
    )
    .map_err(Into::into)
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        MENU_SHOW => {
            let _ = show_main_window(app);
        }
        MENU_HIDE => {
            let _ = hide_main_window(app);
        }
        MENU_START => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = start_runtime_from_tray(app, state.inner());
            }
        }
        MENU_RESTART => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = restart_runtime_from_tray(app, state.inner());
            }
        }
        MENU_STOP => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = state.runtime.stop();
            }
        }
        MENU_EXIT => {
            if let Some(state) = app.try_state::<AppState>() {
                let _ = state.runtime.stop();
            }
            app.exit(0);
            return;
        }
        _ => {}
    }
    let _ = sync_tray(app);
}

fn handle_tray_icon_event<R: Runtime>(app: &AppHandle<R>, event: &TrayIconEvent) {
    match event {
        TrayIconEvent::DoubleClick { .. } => {
            let _ = show_main_window(app);
            let _ = sync_tray(app);
        }
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => {
            let _ = show_main_window(app);
            let _ = sync_tray(app);
        }
        TrayIconEvent::Click {
            button: MouseButton::Right,
            button_state: MouseButtonState::Up,
            ..
        } => {}
        _ => {}
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Ok(())
}

pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
    }
    Ok(())
}

fn start_runtime_from_tray<R: Runtime>(_: &AppHandle<R>, state: &AppState) -> Result<()> {
    let (app_paths, config, prefetched_target) =
        load_runtime_launch_context(state).map_err(|message| anyhow::Error::msg(message.detail))?;
    state
        .runtime
        .start(app_paths, config, prefetched_target)
        .map_err(anyhow::Error::msg)?;
    Ok(())
}

fn restart_runtime_from_tray<R: Runtime>(_: &AppHandle<R>, state: &AppState) -> Result<()> {
    let (app_paths, config, prefetched_target) =
        load_runtime_launch_context(state).map_err(|message| anyhow::Error::msg(message.detail))?;
    state
        .runtime
        .restart(app_paths, config, prefetched_target)
        .map_err(anyhow::Error::msg)?;
    Ok(())
}

fn build_tooltip<R: Runtime>(app: &AppHandle<R>) -> String {
    let status = app
        .try_state::<AppState>()
        .map(|state| state.runtime.snapshot().status)
        .unwrap_or(RuntimeStatus::Idle);
    let window_state = read_window_state(app);
    format!(
        "Autoclick Tauri 2 | {} | {}",
        runtime_status_text(status),
        window_state_text(&window_state)
    )
}

fn runtime_status_text(status: RuntimeStatus) -> &'static str {
    match status {
        RuntimeStatus::Idle => "空闲",
        RuntimeStatus::Starting => "启动中",
        RuntimeStatus::Running => "运行中",
        RuntimeStatus::CoolingDown => "冷却中",
        RuntimeStatus::Recovering => "恢复中",
        RuntimeStatus::Stopping => "停止中",
        RuntimeStatus::Faulted => "故障",
    }
}

#[derive(Clone, Copy)]
struct MainWindowState {
    show_enabled: bool,
    hide_enabled: bool,
}

fn read_window_state<R: Runtime>(app: &AppHandle<R>) -> MainWindowState {
    let Some(window) = app.get_webview_window("main") else {
        return MainWindowState {
            show_enabled: false,
            hide_enabled: false,
        };
    };

    let visible = window.is_visible().unwrap_or(true);
    let minimized = window.is_minimized().unwrap_or(false);
    MainWindowState {
        show_enabled: !visible || minimized,
        hide_enabled: visible && !minimized,
    }
}

fn window_state_text(state: &MainWindowState) -> &'static str {
    match (state.show_enabled, state.hide_enabled) {
        (true, false) => "已隐藏",
        (false, true) => "已显示",
        _ => "不可用",
    }
}
