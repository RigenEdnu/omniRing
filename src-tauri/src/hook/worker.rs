//! Low-level input hook worker thread implementation.
//! Uses Win32 WH_MOUSE_LL in an isolated thread with a message loop (GetMessageW).
//! Events are dispatched non-blocking via bounded crossbeam-channel.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

/// Mouse buttons identifiable by low-level hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1, // Mouse 4 / Back
    X2, // Mouse 5 / Forward
    Other(u16),
}

/// Raw mouse input events captured by the low-level hook.
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
}

impl HookEvent {
    pub fn x(&self) -> i32 {
        match self {
            HookEvent::MouseDown { x, .. }
            | HookEvent::MouseUp { x, .. }
            | HookEvent::MouseMove { x, .. }
            | HookEvent::MouseWheel { x, .. } => *x,
        }
    }

    pub fn y(&self) -> i32 {
        match self {
            HookEvent::MouseDown { y, .. }
            | HookEvent::MouseUp { y, .. }
            | HookEvent::MouseMove { y, .. }
            | HookEvent::MouseWheel { y, .. } => *y,
        }
    }

    pub fn timestamp(&self) -> u32 {
        match self {
            HookEvent::MouseDown { timestamp, .. }
            | HookEvent::MouseUp { timestamp, .. }
            | HookEvent::MouseMove { timestamp, .. }
            | HookEvent::MouseWheel { timestamp, .. } => *timestamp,
        }
    }
}

/// Configuration settings for the low-level hook worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub trigger_button: MouseButton,
    pub suppress_trigger: bool,
    pub track_all_moves: bool,
    pub enabled: bool,
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            trigger_button: MouseButton::X1,
            suppress_trigger: true,
            track_all_moves: false,
            enabled: true,
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

/// Global state accessible by the Win32 HOOKPROC callback.
struct HookGlobalState {
    sender: Sender<HookEvent>,
    trigger_button: RwLock<MouseButton>,
    suppress_trigger: AtomicBool,
    track_all_moves: AtomicBool,
    is_trigger_down: AtomicBool,
    enabled: AtomicBool,
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
            let trigger = state
                .trigger_button
                .read()
                .map(|b| *b)
                .unwrap_or(MouseButton::X1);
            let suppress_enabled = state.suppress_trigger.load(Ordering::Relaxed);

            if let Some(button) = maybe_button {
                if button == trigger {
                    if is_down {
                        state.is_trigger_down.store(true, Ordering::Relaxed);
                    } else {
                        state.is_trigger_down.store(false, Ordering::Relaxed);
                    }

                    if HookDecoder::should_suppress(button, trigger, suppress_enabled) {
                        should_suppress = true;
                    }
                }
            }

            // Only forward move events if trigger is down or track_all_moves is enabled
            let should_send = match event {
                HookEvent::MouseMove { .. } => {
                    state.is_trigger_down.load(Ordering::Relaxed)
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

/// Dedicated background worker managing the low-level mouse hook lifecycle.
pub struct HookWorker {
    thread_handle: Option<std::thread::JoinHandle<()>>,
    thread_id: u32,
    event_receiver: Receiver<HookEvent>,
}

impl HookWorker {
    /// Starts the low-level hook worker thread and begins listening for mouse events.
    pub fn start(config: HookConfig) -> Result<Self, String> {
        let (tx, rx) = crossbeam_channel::bounded(1024);

        {
            let mut state_guard = HOOK_STATE.write().map_err(|e| e.to_string())?;
            *state_guard = Some(HookGlobalState {
                sender: tx,
                trigger_button: RwLock::new(config.trigger_button),
                suppress_trigger: AtomicBool::new(config.suppress_trigger),
                track_all_moves: AtomicBool::new(config.track_all_moves),
                is_trigger_down: AtomicBool::new(false),
                enabled: AtomicBool::new(config.enabled),
            });
        }

        let (init_tx, init_rx) = crossbeam_channel::bounded(1);

        let thread_handle = std::thread::Builder::new()
            .name("omniring-hook-worker".to_string())
            .spawn(move || {
                let thread_id = unsafe { GetCurrentThreadId() };

                let hook_res = unsafe {
                    SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_ll_hook_proc),
                        None,
                        0,
                    )
                };

                match hook_res {
                    Ok(hhook) => {
                        let _ = init_tx.send(Ok(thread_id));

                        let mut msg = MSG::default();
                        // Windows Message Loop on dedicated thread
                        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                            unsafe {
                                let _ = TranslateMessage(&msg);
                                DispatchMessageW(&msg);
                            }
                        }

                        // Cleanup hook on loop termination
                        unsafe {
                            let _ = UnhookWindowsHookEx(hhook);
                        }
                    }
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("SetWindowsHookExW failed: {}", e)));
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

    /// Dynamically reconfigures the active trigger button.
    pub fn set_trigger_button(&self, button: MouseButton) {
        if let Ok(guard) = HOOK_STATE.read() {
            if let Some(state) = guard.as_ref() {
                if let Ok(mut btn) = state.trigger_button.write() {
                    *btn = button;
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
    }

    #[test]
    fn test_hook_config_custom_disabled() {
        let cfg = HookConfig {
            trigger_button: MouseButton::X2,
            suppress_trigger: false,
            track_all_moves: true,
            enabled: false,
        };
        assert_eq!(cfg.trigger_button, MouseButton::X2);
        assert!(!cfg.suppress_trigger);
        assert!(cfg.track_all_moves);
        assert!(!cfg.enabled);
    }
}
