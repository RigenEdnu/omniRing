pub mod action;
pub mod gesture;
pub mod hook;
pub mod profile;
pub mod tray;
pub mod window;

use std::sync::{Mutex, RwLock};
use tauri::{Emitter, Manager, State, WebviewWindow};
use windows::Win32::Foundation::POINT;

/// Application state holding the dedicated low-level input hook worker.
pub struct AppHookState {
    pub worker: Mutex<Option<hook::HookWorker>>,
}

/// Application state holding the dedicated gesture detection worker.
pub struct AppGestureState {
    pub worker: Mutex<Option<gesture::GestureWorker>>,
}

/// Application state holding the profile manager, foreground watcher, and active profile.
pub struct AppProfileState {
    pub manager: RwLock<profile::ProfileManager>,
    pub watcher: Mutex<Option<profile::ForegroundWatcher>>,
    pub active_profile: RwLock<profile::ProfileConfig>,
}

#[tauri::command]
fn show_overlay(
    window: WebviewWindow,
    center_x: i32,
    center_y: i32,
    radius: Option<i32>,
) -> Result<window::RadialPlacement, String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let r = radius.unwrap_or(150);
    window::manager::show_overlay_clamped(
        hwnd,
        POINT {
            x: center_x,
            y: center_y,
        },
        r,
        600,
        600,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_overlay(
    window: WebviewWindow,
    hook_state: State<'_, AppHookState>,
) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    window::manager::hide_overlay(hwnd);
    if let Ok(lock) = hook_state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            worker.set_radial_active(false, 0, 0, 0);
        }
    }
    Ok(())
}

#[tauri::command]
fn get_trigger_mode(
    hook_state: State<'_, AppHookState>,
) -> Result<hook::TriggerMode, String> {
    if let Ok(lock) = hook_state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            return Ok(worker.trigger_mode());
        }
    }
    Err("Hook worker is not running".to_string())
}

#[tauri::command]
fn set_trigger_mode(
    mode: hook::TriggerMode,
    hook_state: State<'_, AppHookState>,
    gesture_state: State<'_, AppGestureState>,
) -> Result<(), String> {
    if let Ok(lock) = hook_state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            worker.set_trigger_mode(mode.clone());
        }
    }
    if let Ok(lock) = gesture_state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            worker.set_trigger_mode(mode);
        }
    }
    Ok(())
}

#[tauri::command]
fn get_cursor_position() -> Result<(i32, i32), String> {
    let pt = window::manager::get_current_cursor_pos().map_err(|e| e.to_string())?;
    Ok((pt.x, pt.y))
}

#[tauri::command]
fn get_monitor_geometry(x: i32, y: i32) -> Result<window::MonitorGeometry, String> {
    window::manager::get_monitor_geometry_from_point(POINT { x, y })
        .ok_or_else(|| "Failed to retrieve monitor geometry for coordinates".to_string())
}

#[tauri::command]
fn calculate_clamped_placement(
    cursor_x: i32,
    cursor_y: i32,
    radius: i32,
    window_width: i32,
    window_height: i32,
) -> Result<window::RadialPlacement, String> {
    let placement = window::manager::get_radial_placement(
        POINT {
            x: cursor_x,
            y: cursor_y,
        },
        radius,
        window_width,
        window_height,
    );
    Ok(placement)
}

#[tauri::command]
fn get_hook_status(state: State<'_, AppHookState>) -> bool {
    if let Ok(lock) = state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            return worker.is_running();
        }
    }
    false
}

#[tauri::command]
fn get_gesture_status(state: State<'_, AppGestureState>) -> bool {
    if let Ok(lock) = state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            return worker.is_running();
        }
    }
    false
}

#[tauri::command]
fn set_hook_trigger_button(
    button_name: String,
    hook_state: State<'_, AppHookState>,
    gesture_state: State<'_, AppGestureState>,
) -> Result<(), String> {
    let button = match button_name.to_lowercase().as_str() {
        "x1" | "back" | "mouse4" => hook::MouseButton::X1,
        "x2" | "forward" | "mouse5" => hook::MouseButton::X2,
        "middle" | "mouse3" => hook::MouseButton::Middle,
        "right" | "mouse2" => hook::MouseButton::Right,
        "left" | "mouse1" => hook::MouseButton::Left,
        _ => return Err(format!("Unknown mouse button: {}", button_name)),
    };

    let mode = hook::TriggerMode::Hold {
        button,
        hold_ms: 150,
    };

    if let Ok(lock) = hook_state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            worker.set_trigger_mode(mode.clone());
        }
    }
    if let Ok(lock) = gesture_state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            worker.set_trigger_mode(mode);
        }
    }
    Ok(())
}

#[tauri::command]
fn set_hook_suppress(
    suppress: bool,
    state: State<'_, AppHookState>,
) -> Result<(), String> {
    if let Ok(lock) = state.worker.lock() {
        if let Some(worker) = lock.as_ref() {
            worker.set_suppress_trigger(suppress);
            return Ok(());
        }
    }
    Err("Hook worker is not running".to_string())
}

#[tauri::command]
fn get_active_profile(
    state: State<'_, AppProfileState>,
) -> Result<profile::ProfileConfig, String> {
    let guard = state.active_profile.read().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
fn get_foreground_process() -> Result<Option<profile::ProcessInfo>, String> {
    Ok(profile::get_current_foreground_process())
}

#[tauri::command]
fn reload_profiles(
    _state: State<'_, AppProfileState>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    tray::reload_profiles_internal(&app)
}

#[tauri::command]
fn open_config_folder() -> Result<(), String> {
    tray::open_config_folder()
}

#[tauri::command]
fn open_configurator_window(app: tauri::AppHandle) -> Result<(), String> {
    tray::open_configurator_window(&app)
}

#[tauri::command]
fn is_omni_enabled(app: tauri::AppHandle) -> bool {
    tray::is_app_enabled(&app)
}

#[tauri::command]
fn toggle_omni_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    tray::toggle_app_enabled(&app)
}

#[tauri::command]
fn set_omni_enabled(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    tray::set_app_enabled(&app, enabled)
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn set_active_profile_by_name(
    name: String,
    state: State<'_, AppProfileState>,
    app: tauri::AppHandle,
) -> Result<profile::ProfileConfig, String> {
    let mgr = state.manager.read().map_err(|e| e.to_string())?;
    let profile = mgr
        .get_profile_by_name(&name)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found", name))?;

    let mut active = state.active_profile.write().map_err(|e| e.to_string())?;
    *active = profile.clone();

    let _ = app.emit("omniring://profile-changed", &profile);
    if let Some(overlay_win) = app.get_webview_window("overlay") {
        let _ = overlay_win.emit("omniring://profile-changed", &profile);
    }

    Ok(profile)
}

#[tauri::command]
fn list_available_profiles(
    state: State<'_, AppProfileState>,
) -> Result<Vec<profile::ProfileConfig>, String> {
    let mgr = state.manager.read().map_err(|e| e.to_string())?;
    Ok(mgr.list_profiles().to_vec())
}

#[tauri::command]
fn save_profile(
    profile: profile::ProfileConfig,
    state: State<'_, AppProfileState>,
    app: tauri::AppHandle,
) -> Result<profile::ProfileConfig, String> {
    let mut mgr = state.manager.write().map_err(|e| e.to_string())?;
    mgr.save_profile(&profile)?;

    if let Ok(mut active) = state.active_profile.write() {
        if active.profile_name == profile.profile_name {
            *active = profile.clone();
        }
    }

    let _ = app.emit("omniring://profile-changed", &profile);
    if let Some(overlay_win) = app.get_webview_window("overlay") {
        let _ = overlay_win.emit("omniring://profile-changed", &profile);
    }

    Ok(profile)
}

#[tauri::command]
fn execute_action(
    action: profile::ActionConfig,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Hide overlay window first for non-submenu actions so the target window receives focus
    if action.action_type != "Submenu" {
        if let Some(overlay_win) = app.get_webview_window("overlay") {
            if let Ok(hwnd) = overlay_win.hwnd() {
                window::manager::hide_overlay(hwnd);
            }
        }
    }

    action::execute_action(&action).map_err(|e| e.to_string())
}

#[tauri::command]
fn execute_slice_action(
    slice_id: String,
    ring_id: Option<String>,
    state: State<'_, AppProfileState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let p_guard = state.active_profile.read().map_err(|e| e.to_string())?;
    let ring = ring_id.as_deref().unwrap_or("root");
    let slices = p_guard
        .get_ring_slices(ring)
        .ok_or_else(|| format!("Ring '{}' not found", ring))?;

    let slice = slices
        .iter()
        .find(|s| s.id == slice_id)
        .ok_or_else(|| format!("Slice '{}' not found in ring '{}'", slice_id, ring))?;

    if let Some(ref act) = slice.action {
        if act.action_type != "Submenu" {
            if let Some(overlay_win) = app.get_webview_window("overlay") {
                if let Ok(hwnd) = overlay_win.hwnd() {
                    window::manager::hide_overlay(hwnd);
                }
            }
        }
        action::execute_action(act).map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

/// Router for foreground application change notifications.
fn handle_foreground_change(app_handle: &tauri::AppHandle, proc_info: profile::ProcessInfo) {
    let _ = app_handle.emit("omniring://foreground-changed", &proc_info);

    if let Some(state) = app_handle.try_state::<AppProfileState>() {
        let matched_profile = if let Ok(mgr_guard) = state.manager.read() {
            mgr_guard.get_profile_for_process(&proc_info.process_name)
        } else {
            return;
        };

        let should_emit = if let Ok(mut active_guard) = state.active_profile.write() {
            if active_guard.profile_name != matched_profile.profile_name {
                *active_guard = matched_profile.clone();
                true
            } else {
                false
            }
        } else {
            false
        };

        if should_emit {
            let _ = app_handle.emit("omniring://profile-changed", &matched_profile);
            if let Some(overlay_win) = app_handle.get_webview_window("overlay") {
                let _ = overlay_win.emit("omniring://profile-changed", &matched_profile);
            }
        }
    }
}

/// Central router for gesture events dispatched by the background gesture detector.
fn handle_gesture_event(app_handle: &tauri::AppHandle, event: gesture::GestureEvent) {
    let _ = app_handle.emit("omniring://gesture", &event);

    match &event {
        gesture::GestureEvent::RadialOpen { center_x, center_y } => {
            if let Some(overlay_win) = app_handle.get_webview_window("overlay") {
                if let Ok(hwnd) = overlay_win.hwnd() {
                    let r = if let Some(p_state) = app_handle.try_state::<AppProfileState>() {
                        p_state
                            .active_profile
                            .read()
                            .map(|p| p.outer_radius as i32)
                            .unwrap_or(150)
                    } else {
                        150
                    };

                    match window::manager::show_overlay_clamped(
                        hwnd,
                        POINT {
                            x: *center_x,
                            y: *center_y,
                        },
                        r,
                        600,
                        600,
                    ) {
                        Ok(placement) => {
                            if let Some(h_state) = app_handle.try_state::<AppHookState>() {
                                if let Ok(lock) = h_state.worker.lock() {
                                    if let Some(worker) = lock.as_ref() {
                                        worker.set_radial_active(
                                            true,
                                            placement.center_x,
                                            placement.center_y,
                                            r,
                                        );
                                    }
                                }
                            }
                            let _ = overlay_win.emit("omniring://radial-open", &placement);
                            return;
                        }
                        Err(e) => {
                            eprintln!("Warning: show_overlay_clamped failed: {}", e);
                        }
                    }
                }
                let _ = overlay_win.emit("omniring://radial-open", &event);
            }
        }
        gesture::GestureEvent::RadialMove { .. } => {
            if let Some(overlay_win) = app_handle.get_webview_window("overlay") {
                let _ = overlay_win.emit("omniring://radial-move", &event);
            }
        }
        gesture::GestureEvent::RadialRelease { .. } => {
            if let Some(h_state) = app_handle.try_state::<AppHookState>() {
                if let Ok(lock) = h_state.worker.lock() {
                    if let Some(worker) = lock.as_ref() {
                        worker.set_radial_active(false, 0, 0, 0);
                    }
                }
            }
            if let Some(overlay_win) = app_handle.get_webview_window("overlay") {
                let _ = overlay_win.emit("omniring://radial-release", &event);
            }
        }
        gesture::GestureEvent::Flick { .. } => {
            let _ = app_handle.emit("omniring://flick", &event);
        }
        gesture::GestureEvent::Tap { button, .. } => {
            // Forward/replay suppressed tap click to system
            gesture::replay_tap_click(*button);
            let _ = app_handle.emit("omniring://tap", &event);
        }
        gesture::GestureEvent::Cancelled => {
            if let Some(h_state) = app_handle.try_state::<AppHookState>() {
                if let Ok(lock) = h_state.worker.lock() {
                    if let Some(worker) = lock.as_ref() {
                        worker.set_radial_active(false, 0, 0, 0);
                    }
                }
            }
            if let Some(overlay_win) = app_handle.get_webview_window("overlay") {
                if let Ok(hwnd) = overlay_win.hwnd() {
                    window::manager::hide_overlay(hwnd);
                }
                let _ = overlay_win.emit("omniring://cancelled", &event);
            }
        }
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize profile manager and resolve profiles directory
    let profiles_dir = profile::config::resolve_profiles_dir();
    let mut profile_mgr = profile::ProfileManager::new(profiles_dir);
    let _ = profile_mgr.load_all();

    // Determine initial active profile
    let initial_proc = profile::get_current_foreground_process();
    let initial_profile = if let Some(ref proc) = initial_proc {
        profile_mgr.get_profile_for_process(&proc.process_name)
    } else {
        profile_mgr.default_profile().clone()
    };

    tauri::Builder::default()
        .manage(AppHookState {
            worker: Mutex::new(None),
        })
        .manage(AppGestureState {
            worker: Mutex::new(None),
        })
        .manage(AppProfileState {
            manager: RwLock::new(profile_mgr),
            watcher: Mutex::new(None),
            active_profile: RwLock::new(initial_profile),
        })
        .manage(tray::AppTrayState::default())
        .setup(|app| {
            if let Some(overlay_win) = app.get_webview_window("overlay") {
                if let Ok(hwnd) = overlay_win.hwnd() {
                    let _ = window::manager::apply_overlay_styles(hwnd);
                }
            }

            // Initialize Windows System Tray and context menu
            if let Err(e) = tray::setup_tray(app) {
                eprintln!("Warning: could not initialize system tray: {}", e);
            }

            // Start dedicated foreground watcher
            match profile::ForegroundWatcher::start() {
                Ok(watcher) => {
                    let watcher_rx = watcher.receiver().clone();
                    let app_handle_profile = app.handle().clone();

                    std::thread::Builder::new()
                        .name("omniring-profile-coordinator".to_string())
                        .spawn(move || {
                            while let Ok(proc_info) = watcher_rx.recv() {
                                handle_foreground_change(&app_handle_profile, proc_info);
                            }
                        })
                        .expect("failed to spawn profile coordinator thread");

                    if let Some(p_state) = app.try_state::<AppProfileState>() {
                        if let Ok(mut lock) = p_state.watcher.lock() {
                            *lock = Some(watcher);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: could not install foreground watcher: {}", e);
                }
            }

            let default_config = hook::HookConfig::default();
            let trigger_mode = default_config.trigger_mode.clone();

            // Start low-level mouse hook worker thread
            match hook::HookWorker::start(default_config) {
                Ok(worker) => {
                    let hook_rx = worker.receiver().clone();

                    if let Some(state) = app.try_state::<AppHookState>() {
                        if let Ok(mut lock) = state.worker.lock() {
                            *lock = Some(worker);
                        }
                    }

                    // Start dedicated gesture detection coordinator
                    let app_handle = app.handle().clone();
                    match gesture::GestureWorker::start(
                        hook_rx,
                        gesture::GestureConfig::default(),
                        trigger_mode,
                        move |event| {
                            handle_gesture_event(&app_handle, event);
                        },
                    ) {
                        Ok(g_worker) => {
                            if let Some(g_state) = app.try_state::<AppGestureState>() {
                                if let Ok(mut lock) = g_state.worker.lock() {
                                    *lock = Some(g_worker);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: could not start gesture worker: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: could not install low-level mouse hook: {}", e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_overlay,
            hide_overlay,
            get_hook_status,
            get_gesture_status,
            get_trigger_mode,
            set_trigger_mode,
            set_hook_trigger_button,
            set_hook_suppress,
            get_cursor_position,
            get_monitor_geometry,
            calculate_clamped_placement,
            get_active_profile,
            get_foreground_process,
            reload_profiles,
            set_active_profile_by_name,
            list_available_profiles,
            save_profile,
            execute_action,
            execute_slice_action,
            open_config_folder,
            open_configurator_window,
            is_omni_enabled,
            toggle_omni_enabled,
            set_omni_enabled,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running omniRing application");
}

