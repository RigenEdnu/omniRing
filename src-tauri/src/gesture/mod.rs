pub mod detector;

pub use detector::{
    is_point_in_corner, replay_tap_click, Direction, GestureConfig, GestureDetector, GestureEvent,
    GestureState,
};

use crate::hook::{HookEvent, TriggerMode};
use crossbeam_channel::Receiver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

/// Dedicated background worker driving the gesture detector state machine.
pub struct GestureWorker {
    thread_handle: Option<JoinHandle<()>>,
    is_running: Arc<AtomicBool>,
    trigger_mode: Arc<RwLock<TriggerMode>>,
}

impl GestureWorker {
    /// Spawns a background coordinator thread that consumes `HookEvent` items from the hook worker,
    /// feeds them into `GestureDetector`, performs timeout evaluations, and forwards `GestureEvent`
    /// results to the supplied callback.
    pub fn start<F>(
        hook_receiver: Receiver<HookEvent>,
        config: GestureConfig,
        trigger_mode: TriggerMode,
        on_gesture: F,
    ) -> Result<Self, String>
    where
        F: Fn(GestureEvent) + Send + 'static,
    {
        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&is_running);
        let trigger_mode_arc = Arc::new(RwLock::new(trigger_mode.clone()));
        let trigger_mode_clone = Arc::clone(&trigger_mode_arc);

        let thread_handle = std::thread::Builder::new()
            .name("omniring-gesture-worker".to_string())
            .spawn(move || {
                let initial_btn = trigger_mode.button().unwrap_or(crate::hook::MouseButton::X1);
                let mut detector = GestureDetector::new(config, initial_btn);
                detector.set_trigger_mode(trigger_mode);

                while running_clone.load(Ordering::Relaxed) {
                    if let Ok(tm) = trigger_mode_clone.read() {
                        if detector.trigger_mode() != &*tm {
                            detector.set_trigger_mode(tm.clone());
                        }
                    }

                    match hook_receiver.recv_timeout(Duration::from_millis(10)) {
                        Ok(hook_event) => {
                            if let Some(event) = detector.handle_hook_event(&hook_event) {
                                on_gesture(event);
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            // Check hold timeout during periods of zero mouse input
                            let now_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u32)
                                .unwrap_or(0);

                            if let Some(event) = detector.check_timeout(now_ms) {
                                on_gesture(event);
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn gesture worker thread: {}", e))?;

        Ok(Self {
            thread_handle: Some(thread_handle),
            is_running,
            trigger_mode: trigger_mode_arc,
        })
    }

    /// Dynamically updates the active trigger mode.
    pub fn set_trigger_mode(&self, mode: TriggerMode) {
        if let Ok(mut tm) = self.trigger_mode.write() {
            *tm = mode;
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for GestureWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

