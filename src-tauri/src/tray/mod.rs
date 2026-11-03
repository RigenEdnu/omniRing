//! System tray and tray menu controls implementation for omniRing.
//! Manages the Windows taskbar notification area icon, right-click context menu,
//! and tray actions (Open Config Folder, Reload Profiles, Toggle Enable/Disable, Exit).

use std::sync::Mutex;
use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, Wry,
};

/// Menu item identifier constants.
pub const MENU_ID_OPEN_CONFIG: &str = "open_config";
pub const MENU_ID_RELOAD_PROFILES: &str = "reload_profiles";
pub const MENU_ID_TOGGLE_ENABLED: &str = "toggle_enabled";
pub const MENU_ID_EXIT: &str = "exit";

/// Managed state for holding tray-related UI components, such as the toggle check menu item.
pub struct AppTrayState {
    pub toggle_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

impl Default for AppTrayState {
    fn default() -> Self {
        Self {
            toggle_item: Mutex::new(None),
        }
    }
}

/// Builds the system tray context menu with all required options:
/// Open Config Folder, Reload Profiles, Enabled (toggle checkmark), and Exit.
pub fn build_tray_menu<R: Runtime, M: Manager<R>>(
    manager: &M,
) -> tauri::Result<(Menu<R>, CheckMenuItem<R>)> {
    let open_config = MenuItemBuilder::with_id(MENU_ID_OPEN_CONFIG, "Open Config Folder")
        .build(manager)?;
    let reload_profiles =
        MenuItemBuilder::with_id(MENU_ID_RELOAD_PROFILES, "Reload Profiles").build(manager)?;
    let toggle_enabled = CheckMenuItemBuilder::with_id(MENU_ID_TOGGLE_ENABLED, "Enabled")
        .checked(true)
        .build(manager)?;
    let exit = MenuItemBuilder::with_id(MENU_ID_EXIT, "Exit").build(manager)?;

    let menu = MenuBuilder::new(manager)
        .item(&open_config)
        .item(&reload_profiles)
        .separator()
        .item(&toggle_enabled)
        .separator()
        .item(&exit)
        .build()?;

    Ok((menu, toggle_enabled))
}

/// Registers and initializes the Windows System Tray icon and its context menu.
pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let (menu, toggle_item) = build_tray_menu(app)?;

    // Store the toggle_item in AppTrayState if managed
    if let Some(tray_state) = app.try_state::<AppTrayState>() {
        if let Ok(mut lock) = tray_state.toggle_item.lock() {
            *lock = Some(toggle_item);
        }
    }

    let mut tray_builder = TrayIconBuilder::with_id("omniring-tray")
        .tooltip("omniRing - Radial Menu & Gesture Ring")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| {
            handle_tray_menu_event(app_handle, event.id().as_ref());
        })
        .on_tray_icon_event(|_tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                // Double-clicking tray icon opens the config folder
                let _ = open_config_folder();
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    let _tray = tray_builder.build(app)?;

    Ok(())
}

/// Dispatches actions triggered by tray menu item activations.
pub fn handle_tray_menu_event<R: Runtime>(app: &AppHandle<R>, menu_id: &str) {
    match menu_id {
        MENU_ID_OPEN_CONFIG => {
            if let Err(e) = open_config_folder() {
                eprintln!("Warning: Failed to open config folder: {}", e);
            }
        }
        MENU_ID_RELOAD_PROFILES => {
            match reload_profiles_internal(app) {
                Ok(count) => {
                    println!("Reloaded {} profile(s) from tray", count);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to reload profiles: {}", e);
                }
            }
        }
        MENU_ID_TOGGLE_ENABLED => {
            match toggle_app_enabled(app) {
                Ok(new_state) => {
                    println!("Toggled omniRing enabled state: {}", new_state);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to toggle enabled state: {}", e);
                }
            }
        }
        MENU_ID_EXIT => {
            app.exit(0);
        }
        _ => {}
    }
}

/// Opens the configuration folder (profiles directory) in Windows File Explorer.
pub fn open_config_folder() -> Result<(), String> {
    let dir = crate::profile::config::resolve_profiles_dir();
    let _ = std::fs::create_dir_all(&dir);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("Failed to launch explorer: {}", e))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    Ok(())
}

/// Reloads profiles and synchronizes the active profile with current foreground window.
pub fn reload_profiles_internal<R: Runtime>(app: &AppHandle<R>) -> Result<usize, String> {
    if let Some(state) = app.try_state::<crate::AppProfileState>() {
        let mut mgr = state.manager.write().map_err(|e| e.to_string())?;
        let count = mgr.load_all()?;

        if let Some(proc_info) = crate::profile::get_current_foreground_process() {
            let matched = mgr.get_profile_for_process(&proc_info.process_name);
            if let Ok(mut active_guard) = state.active_profile.write() {
                *active_guard = matched.clone();
                let _ = app.emit("omniring://profile-changed", &matched);
                if let Some(overlay_win) = app.get_webview_window("overlay") {
                    let _ = overlay_win.emit("omniring://profile-changed", &matched);
                }
            }
        }
        Ok(count)
    } else {
        Err("AppProfileState not registered".to_string())
    }
}

/// Toggles the enabled state of omniRing input hooks and hides active overlay when disabled.
pub fn toggle_app_enabled<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    if let Some(state) = app.try_state::<crate::AppHookState>() {
        if let Ok(lock) = state.worker.lock() {
            if let Some(worker) = lock.as_ref() {
                let current = worker.is_enabled();
                let new_state = !current;
                worker.set_enabled(new_state);

                // If disabled, ensure overlay window is hidden
                if !new_state {
                    if let Some(overlay_win) = app.get_webview_window("overlay") {
                        if let Ok(hwnd) = overlay_win.hwnd() {
                            crate::window::manager::hide_overlay(hwnd);
                        }
                    }
                }

                let _ = app.emit("omniring://enabled-changed", new_state);
                if let Some(overlay_win) = app.get_webview_window("overlay") {
                    let _ = overlay_win.emit("omniring://enabled-changed", new_state);
                }

                // Synchronize CheckMenuItem in tray
                if let Some(tray_state) = app.try_state::<AppTrayState>() {
                    if let Ok(item_lock) = tray_state.toggle_item.lock() {
                        if let Some(item) = item_lock.as_ref() {
                            let _ = item.set_checked(new_state);
                        }
                    }
                }

                return Ok(new_state);
            }
        }
        Err("Hook worker is not active".to_string())
    } else {
        Err("AppHookState not registered".to_string())
    }
}

/// Explicitly sets the enabled state of omniRing input hooks.
pub fn set_app_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), String> {
    if let Some(state) = app.try_state::<crate::AppHookState>() {
        if let Ok(lock) = state.worker.lock() {
            if let Some(worker) = lock.as_ref() {
                worker.set_enabled(enabled);

                if !enabled {
                    if let Some(overlay_win) = app.get_webview_window("overlay") {
                        if let Ok(hwnd) = overlay_win.hwnd() {
                            crate::window::manager::hide_overlay(hwnd);
                        }
                    }
                }

                let _ = app.emit("omniring://enabled-changed", enabled);
                if let Some(overlay_win) = app.get_webview_window("overlay") {
                    let _ = overlay_win.emit("omniring://enabled-changed", enabled);
                }

                if let Some(tray_state) = app.try_state::<AppTrayState>() {
                    if let Ok(item_lock) = tray_state.toggle_item.lock() {
                        if let Some(item) = item_lock.as_ref() {
                            let _ = item.set_checked(enabled);
                        }
                    }
                }

                return Ok(());
            }
        }
        Err("Hook worker is not active".to_string())
    } else {
        Err("AppHookState not registered".to_string())
    }
}

/// Checks whether omniRing input hooks are currently enabled.
pub fn is_app_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    if let Some(state) = app.try_state::<crate::AppHookState>() {
        if let Ok(lock) = state.worker.lock() {
            if let Some(worker) = lock.as_ref() {
                return worker.is_enabled();
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_menu_ids() {
        assert_eq!(MENU_ID_OPEN_CONFIG, "open_config");
        assert_eq!(MENU_ID_RELOAD_PROFILES, "reload_profiles");
        assert_eq!(MENU_ID_TOGGLE_ENABLED, "toggle_enabled");
        assert_eq!(MENU_ID_EXIT, "exit");
    }

    #[test]
    fn test_app_tray_state_default() {
        let state = AppTrayState::default();
        let lock = state.toggle_item.lock().unwrap();
        assert!(lock.is_none());
    }

    #[test]
    fn test_resolve_profiles_dir_valid() {
        let dir = crate::profile::config::resolve_profiles_dir();
        assert!(!dir.as_os_str().is_empty());
    }
}
