//! Low-level input hook worker thread implementation.
//! Uses Win32 WH_MOUSE_LL and WH_KEYBOARD_LL in an isolated thread with a message loop (GetMessageW).
//! Events are dispatched non-blocking via bounded crossbeam-channel.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::RwLock;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

/// Mouse buttons identifiable by low-level hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    #[serde(alias = "left", alias = "mouse1")]
    Left,
    #[serde(alias = "right", alias = "mouse2")]
    Right,
    #[serde(alias = "middle", alias = "mouse3")]
    Middle,
    #[serde(alias = "x1", alias = "back", alias = "mouse4")]
    X1, // Mouse 4 / Back
    #[serde(alias = "x2", alias = "forward", alias = "mouse5")]
    X2, // Mouse 5 / Forward
    Other(u16),
}

fn default_hold_ms() -> u32 {
    150
}

fn default_dwell_ms() -> u32 {
    300
}

/// Trigger modes supported by omniRing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum TriggerMode {
    #[serde(rename = "hold", alias = "Hold")]
    Hold {
        button: MouseButton,
        #[serde(default = "default_hold_ms")]
        hold_ms: u32,
    },
    #[serde(rename = "combo", alias = "Combo")]
    Combo {
        modifiers: Vec<String>,
        button: MouseButton,
    },
    #[serde(rename = "cornerhover", alias = "CornerHover", alias = "corner_hover")]
    CornerHover {
        corner: String,
        #[serde(default = "default_dwell_ms")]
        dwell_ms: u32,
    },
}

impl Default for TriggerMode {
    fn default() -> Self {
        TriggerMode::Hold {
            button: MouseButton::X1,
            hold_ms: 150,
        }
    }
}

impl TriggerMode {
    /// Returns the mouse button associated with this trigger mode, if any.
    pub fn button(&self) -> Option<MouseButton> {
        match self {
            TriggerMode::Hold { button, .. } => Some(*button),
            TriggerMode::Combo { button, .. } => Some(*button),
            TriggerMode::CornerHover { .. } => None,
        }
    }
}

/// Checks if a named modifier key is currently pressed using GetAsyncKeyState.
pub fn is_modifier_pressed(modifier: &str) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let vkey: i32 = match modifier.to_lowercase().as_str() {
            "ctrl" | "control" => 0x11,     // VK_CONTROL
            "lctrl" => 0xA2,                // VK_LCONTROL
            "rctrl" => 0xA3,                // VK_RCONTROL
            "shift" => 0x10,               // VK_SHIFT
            "lshift" => 0xA0,              // VK_LSHIFT
            "rshift" => 0xA1,              // VK_RSHIFT
            "alt" | "menu" => 0x12,        // VK_MENU
            "lalt" => 0xA4,                // VK_LMENU
            "ralt" => 0xA5,                // VK_RMENU
            "win" | "windows" | "super" | "meta" | "lwin" => 0x5B, // VK_LWIN
            "rwin" => 0x5C,                // VK_RWIN
            _ => return false,
        };
        (GetAsyncKeyState(vkey) as u16 & 0x8000) != 0
    }
    #[cfg(not(windows))]
    {
        let _ = modifier;
        false
    }
}

/// Checks if all specified modifier keys are currently pressed.
pub fn are_modifiers_pressed(modifiers: &[String]) -> bool {
    if modifiers.is_empty() {
        return true;
    }
    modifiers.iter().all(|m| is_modifier_pressed(m))
}

/// Raw mouse and keyboard input events captured by the low-level hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookEvent {
    MouseDown {
        button: MouseButton,
        x: i32,
        y: i32,
        timestamp: u32,
    },
    MouseUp {
        button: MouseButton,
        x: i32,
        y: i32,
        timestamp: u32,
    },
    MouseMove {
        x: i32,
        y: i32,
        timestamp: u32,
    },
    MouseWheel {
        delta: i16,
        x: i32,
        y: i32,
        timestamp: u32,
    },
    RadialCancel,
    OutsideClick {
        x: i32,
        y: i32,
    },
}

impl HookEvent {
    pub fn x(&self) -> i32 {
        match self {
            HookEvent::MouseDown { x, .. }
            | HookEvent::MouseUp { x, .. }
            | HookEvent::MouseMove { x, .. }
            | HookEvent::MouseWheel { x, .. }
            | HookEvent::OutsideClick { x, .. } => *x,
            HookEvent::RadialCancel => 0,
        }
    }

    pub fn y(&self) -> i32 {
        match self {
            HookEvent::MouseDown { y, .. }
            | HookEvent::MouseUp { y, .. }
            | HookEvent::MouseMove { y, .. }
            | HookEvent::MouseWheel { y, .. }
            | HookEvent::OutsideClick { y, .. } => *y,
            HookEvent::RadialCancel => 0,
        }
    }

    pub fn timestamp(&self) -> u32 {
        match self {
            HookEvent::MouseDown { timestamp, .. }
            | HookEvent::MouseUp { timestamp, .. }
            | HookEvent::MouseMove { timestamp, .. }
            | HookEvent::MouseWheel { timestamp, .. } => *timestamp,
            HookEvent::RadialCancel | HookEvent::OutsideClick { .. } => 0,
        }
    }
}

fn default_outer_radius() -> i32 {
    150
}

fn default_tolerance() -> i32 {
    40
}

/// Configuration settings for the low-level hook worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub trigger_button: MouseButton,
    pub suppress_trigger: bool,
    pub track_all_moves: bool,
    pub enabled: bool,
    #[serde(default)]
    pub trigger_mode: TriggerMode,
    #[serde(default = "default_outer_radius")]
    pub outer_radius: i32,
    #[serde(default = "default_tolerance")]
    pub outside_tolerance: i32,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            trigger_button: MouseButton::X1,
            suppress_trigger: true,
            track_all_moves: false,
            enabled: true,
            trigger_mode: TriggerMode::default(),
            outer_radius: 150,
            outside_tolerance: 40,
        }
    }
}

/// Decoder for Win32 mouse messages into high-level HookEvent items.
pub struct HookDecoder;

impl HookDecoder {
    /// Decodes a low-level Win32 mouse message into an optional HookEvent,
    /// associated button (if down/up), and a boolean indicating if it is a press event.
    pub fn decode_message(
        msg: u32,
        x: i32,
        y: i32,
        mouse_data: u32,
        timestamp: u32,
    ) -> Option<(HookEvent, Option<MouseButton>, bool)> {
        match msg {
            WM_LBUTTONDOWN => Some((
                HookEvent::MouseDown {
                    button: MouseButton::Left,
                    x,
                    y,
                    timestamp,
                },
                Some(MouseButton::Left),
                true,
            )),
            WM_LBUTTONUP => Some((
                HookEvent::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                    timestamp,
                },
                Some(MouseButton::Left),
                false,
            )),
            WM_RBUTTONDOWN => Some((
                HookEvent::MouseDown {
                    button: MouseButton::Right,
                    x,
                    y,
                    timestamp,
                },
                Some(MouseButton::Right),
                true,
            )),
            WM_RBUTTONUP => Some((
                HookEvent::MouseUp {
                    button: MouseButton::Right,
                    x,
                    y,
                    timestamp,
                },
                Some(MouseButton::Right),
                false,
            )),
            WM_MBUTTONDOWN => Some((
                HookEvent::MouseDown {
                    button: MouseButton::Middle,
                    x,
                    y,
                    timestamp,
                },
                Some(MouseButton::Middle),
                true,
            )),
            WM_MBUTTONUP => Some((
                HookEvent::MouseUp {
                    button: MouseButton::Middle,
                    x,
                    y,
                    timestamp,
                },
                Some(MouseButton::Middle),
                false,
            )),
            WM_XBUTTONDOWN => {
                let xbutton = (mouse_data >> 16) as u16;
                let btn = match xbutton {
                    1 => MouseButton::X1,
                    2 => MouseButton::X2,
                    other => MouseButton::Other(other),
                };
                Some((
                    HookEvent::MouseDown {
                        button: btn,
                        x,
                        y,
                        timestamp,
                    },
                    Some(btn),
                    true,
                ))
            }
            WM_XBUTTONUP => {
                let xbutton = (mouse_data >> 16) as u16;
                let btn = match xbutton {
                    1 => MouseButton::X1,
                    2 => MouseButton::X2,
                    other => MouseButton::Other(other),
                };
                Some((
                    HookEvent::MouseUp {
                        button: btn,
                        x,
                        y,
                        timestamp,
                    },
                    Some(btn),
                    false,
                ))
            }
            WM_MOUSEMOVE => Some((
                HookEvent::MouseMove { x, y, timestamp },
                None,
                false,
            )),
            WM_MOUSEWHEEL => {
                let delta = (mouse_data >> 16) as i16;
                Some((
                    HookEvent::MouseWheel {
                        delta,
                        x,
                        y,
                        timestamp,
                    },
                    None,
                    false,
                ))
            }
            _ => None,
        }
    }

    /// Determines whether a button event should be suppressed (blocked from target windows).
    pub fn should_suppress(
        button: MouseButton,
        trigger_button: MouseButton,
        suppress_enabled: bool,
    ) -> bool {
        suppress_enabled && (button == trigger_button)
    }
}

/// Global state accessible by the Win32 HOOKPROC callbacks.
struct HookGlobalState {
    sender: Sender<HookEvent>,
    trigger_mode: RwLock<TriggerMode>,
    trigger_button: RwLock<MouseButton>,
    suppress_trigger: AtomicBool,
    track_all_moves: AtomicBool,
    is_trigger_down: AtomicBool,
    enabled: AtomicBool,
    radial_active: AtomicBool,
    radial_center_x: AtomicI32,
    radial_center_y: AtomicI32,
    radial_outer_radius: AtomicI32,
    radial_tolerance: AtomicI32,
}

static HOOK_STATE: RwLock<Option<HookGlobalState>> = RwLock::new(None);

/// Win32 low-level mouse hook procedure.
/// Must finish rapidly (< few microseconds) to avoid Windows hook timeouts.
unsafe extern "system" fn mouse_ll_hook_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code < 0 || lparam.0 == 0 {
        return CallNextHookEx(None, n_code, wparam, lparam);
    }

    let hook_struct = &*(lparam.0 as *const MSLLHOOKSTRUCT);
    let msg = wparam.0 as u32;

    // Fast path: bypass synthetic injected events (e.g. from SendInput / mouse_event)
    // to prevent infinite loops and ensure replayed tap clicks pass to target windows.
    let is_injected = (hook_struct.flags & 1) != 0;
    if is_injected {
        return CallNextHookEx(None, n_code, wparam, lparam);
    }

    // If hook is globally disabled, pass all events through to next hook
    if let Ok(guard) = HOOK_STATE.read() {
        if let Some(state) = guard.as_ref() {
            if !state.enabled.load(Ordering::Relaxed) {
                return CallNextHookEx(None, n_code, wparam, lparam);
            }
        }
    }

    let (event, maybe_button, is_down) = match HookDecoder::decode_message(
        msg,
        hook_struct.pt.x,
        hook_struct.pt.y,
        hook_struct.mouseData,
        hook_struct.time,
    ) {
        Some(decoded) => decoded,
        None => return CallNextHookEx(None, n_code, wparam, lparam),
    };

    let mut should_suppress = false;

    if let Ok(guard) = HOOK_STATE.read() {
        if let Some(state) = guard.as_ref() {
            let mode = state
                .trigger_mode
                .read()
                .map(|m| m.clone())
                .unwrap_or_default();
            let suppress_enabled = state.suppress_trigger.load(Ordering::Relaxed);
            let radial_active = state.radial_active.load(Ordering::Relaxed);

            // Outside click auto-close check
            if radial_active && is_down {
                let center_x = state.radial_center_x.load(Ordering::Relaxed);
                let center_y = state.radial_center_y.load(Ordering::Relaxed);
                let outer_radius = state.radial_outer_radius.load(Ordering::Relaxed);
                let tolerance = state.radial_tolerance.load(Ordering::Relaxed);
                let max_radius = (outer_radius + tolerance) as f64;

                let dx = hook_struct.pt.x - center_x;
                let dy = hook_struct.pt.y - center_y;
                let dist = ((dx * dx + dy * dy) as f64).sqrt();

                if dist > max_radius {
                    // Click occurred outside radial menu boundary!
                    state.radial_active.store(false, Ordering::Relaxed);
                    state.is_trigger_down.store(false, Ordering::Relaxed);
                    let _ = state.sender.try_send(HookEvent::OutsideClick {
                        x: hook_struct.pt.x,
                        y: hook_struct.pt.y,
                    });
                    // Do NOT suppress: let click reach underlying window
                    return CallNextHookEx(None, n_code, wparam, lparam);
                }
            }

            // Trigger mode handling
            let mut forward_event = true;
            match &mode {
                TriggerMode::Hold { button, .. } => {
                    if let Some(btn) = maybe_button {
                        if btn == *button {
                            if is_down {
                                state.is_trigger_down.store(true, Ordering::Relaxed);
                            } else {
                                state.is_trigger_down.store(false, Ordering::Relaxed);
                            }
                            if HookDecoder::should_suppress(btn, *button, suppress_enabled) {
                                should_suppress = true;
                            }
                        }
                    }
                }
                TriggerMode::Combo { modifiers, button } => {
                    if let Some(btn) = maybe_button {
                        if btn == *button {
                            if is_down {
                                if are_modifiers_pressed(modifiers) {
                                    state.is_trigger_down.store(true, Ordering::Relaxed);
                                    if suppress_enabled {
                                        should_suppress = true;
                                    }
                                } else {
                                    // Modifiers not pressed: ordinary mouse click passes through unaffected!
                                    state.is_trigger_down.store(false, Ordering::Relaxed);
                                    forward_event = false;
                                }
                            } else {
                                // Mouse up: if trigger was down, suppress up as well
                                if state.is_trigger_down.load(Ordering::Relaxed) {
                                    state.is_trigger_down.store(false, Ordering::Relaxed);
                                    if suppress_enabled {
                                        should_suppress = true;
                                    }
                                } else {
                                    forward_event = false;
                                }
                            }
                        }
                    }
                }
                TriggerMode::CornerHover { .. } => {
                    // CornerHover doesn't suppress mouse buttons
                }
            }

            // Only forward move events if trigger is down, radial is active, or track_all_moves is enabled
            let should_send = forward_event && match event {
                HookEvent::MouseMove { .. } => {
                    state.is_trigger_down.load(Ordering::Relaxed)
                        || state.radial_active.load(Ordering::Relaxed)
                        || state.track_all_moves.load(Ordering::Relaxed)
                }
                _ => true,
            };

            if should_send {
                // Non-blocking send: drops if channel is saturated to guarantee hook safety
                let _ = state.sender.try_send(event);
            }
        }
    }

    if should_suppress {
        LRESULT(1)
    } else {
        CallNextHookEx(None, n_code, wparam, lparam)
    }
}

/// Win32 low-level keyboard hook procedure.
/// Intercepts VK_ESCAPE (0x1B) when the radial menu is active, dismisses it, and suppresses the key event.
unsafe extern "system" fn keyboard_ll_hook_proc(
    n_code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if n_code < 0 || lparam.0 == 0 {
        return CallNextHookEx(None, n_code, wparam, lparam);
    }

    let msg = wparam.0 as u32;
    if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
        let kbd_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        // VK_ESCAPE is 0x1B (27)
        if kbd_struct.vkCode == 0x1B {
            if let Ok(guard) = HOOK_STATE.read() {
                if let Some(state) = guard.as_ref() {
                    if state.enabled.load(Ordering::Relaxed)
                        && state.radial_active.load(Ordering::Relaxed)
                    {
                        // Dismiss radial menu: reset active flags and send RadialCancel
                        state.radial_active.store(false, Ordering::Relaxed);
                        state.is_trigger_down.store(false, Ordering::Relaxed);
                        let _ = state.sender.try_send(HookEvent::RadialCancel);

                        // Suppress Esc so underlying window does not receive it
                        return LRESULT(1);
                    }
                }
            }
        }
    }

    CallNextHookEx(None, n_code, wparam, lparam)
}

/// Dedicated background worker managing the low-level mouse and keyboard hooks lifecycle.
pub struct HookWorker {
    thread_handle: Option<std::thread::JoinHandle<()>>,
    thread_id: u32,
    event_receiver: Receiver<HookEvent>,
}

impl HookWorker {
    /// Starts the low-level hook worker thread and begins listening for mouse and keyboard events.
    pub fn start(config: HookConfig) -> Result<Self, String> {
        let (tx, rx) = crossbeam_channel::bounded(1024);

        {
            let mut state_guard = HOOK_STATE.write().map_err(|e| e.to_string())?;
            *state_guard = Some(HookGlobalState {
                sender: tx,
                trigger_mode: RwLock::new(config.trigger_mode.clone()),
                trigger_button: RwLock::new(config.trigger_button),
                suppress_trigger: AtomicBool::new(config.suppress_trigger),
                track_all_moves: AtomicBool::new(config.track_all_moves),
                is_trigger_down: AtomicBool::new(false),
                enabled: AtomicBool::new(config.enabled),
                radial_active: AtomicBool::new(false),
                radial_center_x: AtomicI32::new(0),
                radial_center_y: AtomicI32::new(0),
                radial_outer_radius: AtomicI32::new(config.outer_radius),
                radial_tolerance: AtomicI32::new(config.outside_tolerance),
            });
        }

        let (init_tx, init_rx) = crossbeam_channel::bounded(1);

        let thread_handle = std::thread::Builder::new()
            .name("omniring-hook-worker".to_string())
            .spawn(move || {
                let thread_id = unsafe { GetCurrentThreadId() };

                let mouse_hook_res = unsafe {
                    SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_ll_hook_proc),
                        None,
                        0,
                    )
                };

                let kbd_hook_res = unsafe {
                    SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(keyboard_ll_hook_proc),
                        None,
                        0,
                    )
                };

                match mouse_hook_res {
                    Ok(m_hook) => {
                        let _ = init_tx.send(Ok(thread_id));

                        let mut msg = MSG::default();
                        // Windows Message Loop on dedicated thread
                        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                            unsafe {
                                let _ = TranslateMessage(&msg);
                                DispatchMessageW(&msg);
                            }
                        }

                        // Cleanup hooks on loop termination
                        unsafe {
                            let _ = UnhookWindowsHookEx(m_hook);
                            if let Ok(k_hook) = kbd_hook_res {
                                let _ = UnhookWindowsHookEx(k_hook);
                            }
                        }
                    }
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("SetWindowsHookExW WH_MOUSE_LL failed: {}", e)));
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn hook thread: {}", e))?;

        let thread_id = match init_rx.recv() {
            Ok(Ok(id)) => id,
            Ok(Err(err)) => {
                if let Ok(mut state_guard) = HOOK_STATE.write() {
                    *state_guard = None;
                }
                let _ = thread_handle.join();
                return Err(err);
            }
            Err(e) => {
                if let Ok(mut state_guard) = HOOK_STATE.write() {
                    *state_guard = None;
                }
                return Err(format!("Init channel disconnected: {}", e));
            }
        };

        Ok(Self {
            thread_handle: Some(thread_handle),
            thread_id,
            event_receiver: rx,
        })
    }

    /// Returns a reference to the event receiver channel.
    pub fn receiver(&self) -> &Receiver<HookEvent> {
        &self.event_receiver
    }

    /// Returns the active trigger mode.
    pub fn trigger_mode(&self) -> TriggerMode {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                if let Ok(tm) = state.trigger_mode.read() {
                    return tm.clone();
                }
            }
        }
        TriggerMode::default()
    }

    /// Dynamically reconfigures the trigger mode.
    pub fn set_trigger_mode(&self, mode: TriggerMode) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                if let Some(btn) = mode.button() {
                    if let Ok(mut b) = state.trigger_button.write() {
                        *b = btn;
                    }
                }
                match &mode {
                    TriggerMode::CornerHover { .. } => {
                        state.track_all_moves.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }
                if let Ok(mut tm) = state.trigger_mode.write() {
                    *tm = mode;
                }
            }
        }
    }

    /// Dynamically reconfigures the active trigger button.
    pub fn set_trigger_button(&self, button: MouseButton) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                if let Ok(mut btn) = state.trigger_button.write() {
                    *btn = button;
                }
                if let Ok(mut tm) = state.trigger_mode.write() {
                    *tm = TriggerMode::Hold {
                        button,
                        hold_ms: 150,
                    };
                }
            }
        }
    }

    /// Dynamically toggles click suppression on the trigger button.
    pub fn set_suppress_trigger(&self, suppress: bool) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                state.suppress_trigger.store(suppress, Ordering::Relaxed);
            }
        }
    }

    /// Dynamically toggles whether all mouse moves are delivered.
    pub fn set_track_all_moves(&self, track: bool) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                state.track_all_moves.store(track, Ordering::Relaxed);
            }
        }
    }

    /// Returns whether the trigger button is currently held down.
    pub fn is_trigger_down(&self) -> bool {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                return state.is_trigger_down.load(Ordering::Relaxed);
            }
        }
        false
    }

    /// Sets the active radial overlay geometry and state for outside click and Esc auto-close.
    pub fn set_radial_active(&self, active: bool, center_x: i32, center_y: i32, outer_radius: i32) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                state.radial_active.store(active, Ordering::Relaxed);
                state.radial_center_x.store(center_x, Ordering::Relaxed);
                state.radial_center_y.store(center_y, Ordering::Relaxed);
                state.radial_outer_radius.store(outer_radius, Ordering::Relaxed);
            }
        }
    }

    /// Checks if the radial overlay is currently marked active in hook state.
    pub fn is_radial_active(&self) -> bool {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                return state.radial_active.load(Ordering::Relaxed);
            }
        }
        false
    }

    /// Checks if the low-level hook is currently enabled.
    pub fn is_enabled(&self) -> bool {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                return state.enabled.load(Ordering::Relaxed);
            }
        }
        true
    }

    /// Dynamically toggles the enabled state of the low-level hook.
    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                state.enabled.store(enabled, Ordering::Relaxed);
                if !enabled {
                    state.is_trigger_down.store(false, Ordering::Relaxed);
                    state.radial_active.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    /// Returns whether the worker thread is currently running.
    pub fn is_running(&self) -> bool {
        self.thread_handle.is_some() && self.thread_id != 0
    }

    /// Stops the worker thread cleanly by posting WM_QUIT and unhooking.
    pub fn stop(&mut self) {
        if self.thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(
                    self.thread_id,
                    WM_QUIT,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
            self.thread_id = 0;
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        if let Ok(mut state_guard) = HOOK_STATE.write() {
            *state_guard = None;
        }
    }
}

impl Drop for HookWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_mouse_move() {
        let decoded = HookDecoder::decode_message(WM_MOUSEMOVE, 100, 200, 0, 1000);
        assert!(decoded.is_some());
        let (event, button, is_down) = decoded.unwrap();
        assert_eq!(
            event,
            HookEvent::MouseMove {
                x: 100,
                y: 200,
                timestamp: 1000
            }
        );
        assert_eq!(button, None);
        assert!(!is_down);
        assert_eq!(event.x(), 100);
        assert_eq!(event.y(), 200);
        assert_eq!(event.timestamp(), 1000);
    }

    #[test]
    fn test_decode_left_button() {
        let down = HookDecoder::decode_message(WM_LBUTTONDOWN, 50, 75, 0, 1050).unwrap();
        assert_eq!(
            down.0,
            HookEvent::MouseDown {
                button: MouseButton::Left,
                x: 50,
                y: 75,
                timestamp: 1050
            }
        );
        assert_eq!(down.1, Some(MouseButton::Left));
        assert!(down.2);

        let up = HookDecoder::decode_message(WM_LBUTTONUP, 50, 75, 0, 1150).unwrap();
        assert_eq!(
            up.0,
            HookEvent::MouseUp {
                button: MouseButton::Left,
                x: 50,
                y: 75,
                timestamp: 1150
            }
        );
        assert_eq!(up.1, Some(MouseButton::Left));
        assert!(!up.2);
    }

    #[test]
    fn test_decode_right_button() {
        let down = HookDecoder::decode_message(WM_RBUTTONDOWN, 30, 40, 0, 2000).unwrap();
        assert_eq!(down.1, Some(MouseButton::Right));
        assert!(down.2);

        let up = HookDecoder::decode_message(WM_RBUTTONUP, 30, 40, 0, 2100).unwrap();
        assert_eq!(up.1, Some(MouseButton::Right));
        assert!(!up.2);
    }

    #[test]
    fn test_decode_middle_button() {
        let down = HookDecoder::decode_message(WM_MBUTTONDOWN, 10, 20, 0, 3000).unwrap();
        assert_eq!(down.1, Some(MouseButton::Middle));
        assert!(down.2);

        let up = HookDecoder::decode_message(WM_MBUTTONUP, 10, 20, 0, 3100).unwrap();
        assert_eq!(up.1, Some(MouseButton::Middle));
        assert!(!up.2);
    }

    #[test]
    fn test_decode_x1_x2_buttons() {
        let down_x1 = HookDecoder::decode_message(WM_XBUTTONDOWN, 100, 150, 1 << 16, 4000).unwrap();
        assert_eq!(
            down_x1.0,
            HookEvent::MouseDown {
                button: MouseButton::X1,
                x: 100,
                y: 150,
                timestamp: 4000
            }
        );
        assert_eq!(down_x1.1, Some(MouseButton::X1));
        assert!(down_x1.2);

        let up_x1 = HookDecoder::decode_message(WM_XBUTTONUP, 100, 150, 1 << 16, 4100).unwrap();
        assert_eq!(up_x1.1, Some(MouseButton::X1));
        assert!(!up_x1.2);

        let down_x2 = HookDecoder::decode_message(WM_XBUTTONDOWN, 120, 180, 2 << 16, 5000).unwrap();
        assert_eq!(
            down_x2.0,
            HookEvent::MouseDown {
                button: MouseButton::X2,
                x: 120,
                y: 180,
                timestamp: 5000
            }
        );
        assert_eq!(down_x2.1, Some(MouseButton::X2));
        assert!(down_x2.2);

        let up_x2 = HookDecoder::decode_message(WM_XBUTTONUP, 120, 180, 2 << 16, 5100).unwrap();
        assert_eq!(up_x2.1, Some(MouseButton::X2));
        assert!(!up_x2.2);
    }

    #[test]
    fn test_decode_wheel() {
        let wheel_up = HookDecoder::decode_message(WM_MOUSEWHEEL, 200, 300, 0x00780000, 6000).unwrap();
        assert_eq!(
            wheel_up.0,
            HookEvent::MouseWheel {
                delta: 120,
                x: 200,
                y: 300,
                timestamp: 6000
            }
        );
        assert_eq!(wheel_up.1, None);

        let raw_neg = ((-120i16 as u16 as u32) << 16) as u32;
        let wheel_down = HookDecoder::decode_message(WM_MOUSEWHEEL, 200, 300, raw_neg, 6050).unwrap();
        if let HookEvent::MouseWheel { delta, .. } = wheel_down.0 {
            assert_eq!(delta, -120);
        } else {
            panic!("Expected MouseWheel");
        }
    }

    #[test]
    fn test_suppress_rules() {
        assert!(HookDecoder::should_suppress(
            MouseButton::X1,
            MouseButton::X1,
            true
        ));
        assert!(!HookDecoder::should_suppress(
            MouseButton::Left,
            MouseButton::X1,
            true
        ));
        assert!(!HookDecoder::should_suppress(
            MouseButton::X1,
            MouseButton::X1,
            false
        ));
    }

    #[test]
    fn test_hook_config_default() {
        let cfg = HookConfig::default();
        assert_eq!(cfg.trigger_button, MouseButton::X1);
        assert!(cfg.suppress_trigger);
        assert!(!cfg.track_all_moves);
        assert!(cfg.enabled);
        assert_eq!(cfg.outer_radius, 150);
        assert_eq!(cfg.outside_tolerance, 40);
        assert_eq!(cfg.trigger_mode, TriggerMode::Hold { button: MouseButton::X1, hold_ms: 150 });
    }

    #[test]
    fn test_hook_config_custom_disabled() {
        let cfg = HookConfig {
            trigger_button: MouseButton::X2,
            suppress_trigger: false,
            track_all_moves: true,
            enabled: false,
            trigger_mode: TriggerMode::Hold {
                button: MouseButton::X2,
                hold_ms: 200,
            },
            outer_radius: 180,
            outside_tolerance: 50,
        };
        assert_eq!(cfg.trigger_button, MouseButton::X2);
        assert!(!cfg.suppress_trigger);
        assert!(cfg.track_all_moves);
        assert!(!cfg.enabled);
        assert_eq!(cfg.outer_radius, 180);
        assert_eq!(cfg.outside_tolerance, 50);
    }

    #[test]
    fn test_trigger_mode_serde() {
        // Hold
        let hold = TriggerMode::Hold {
            button: MouseButton::Middle,
            hold_ms: 180,
        };
        let hold_json = serde_json::to_string(&hold).unwrap();
        let hold_parsed: TriggerMode = serde_json::from_str(&hold_json).unwrap();
        assert_eq!(hold, hold_parsed);

        // Combo
        let combo = TriggerMode::Combo {
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            button: MouseButton::Right,
        };
        let combo_json = serde_json::to_string(&combo).unwrap();
        let combo_parsed: TriggerMode = serde_json::from_str(&combo_json).unwrap();
        assert_eq!(combo, combo_parsed);

        // CornerHover
        let corner = TriggerMode::CornerHover {
            corner: "top-left".to_string(),
            dwell_ms: 250,
        };
        let corner_json = serde_json::to_string(&corner).unwrap();
        let corner_parsed: TriggerMode = serde_json::from_str(&corner_json).unwrap();
        assert_eq!(corner, corner_parsed);
    }

    #[test]
    fn test_hook_event_helpers() {
        let cancel = HookEvent::RadialCancel;
        assert_eq!(cancel.x(), 0);
        assert_eq!(cancel.y(), 0);
        assert_eq!(cancel.timestamp(), 0);

        let outside = HookEvent::OutsideClick { x: 500, y: 600 };
        assert_eq!(outside.x(), 500);
        assert_eq!(outside.y(), 600);
        assert_eq!(outside.timestamp(), 0);
    }

    #[test]
    fn test_are_modifiers_pressed_empty() {
        assert!(are_modifiers_pressed(&[]));
    }
}
