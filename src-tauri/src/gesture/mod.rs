pub mod detector;

pub use detector::{
    replay_tap_click, Direction, GestureConfig, GestureDetector, GestureEvent, GestureState,
};

use crate::hook::HookEvent;
use crossbeam_channel::Receiver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// Dedicated background worker driving the gesture detector state machine.
pub struct GestureWorker {
    thread_handle: Option<JoinHandle<()>>,
    is_running: Arc<AtomicBool>,
}

impl GestureWorker {
    /// Spawns a background coordinator thread that consumes `HookEvent` items from the hook worker,
    /// feeds them into `GestureDetector`, performs timeout evaluations, and forwards `GestureEvent`
    /// results to the supplied callback.
    pub fn start<F>(
        hook_receiver: Receiver<HookEvent>,
        config: GestureConfig,
        trigger_button: crate::hook::MouseButton,
        on_gesture: F,
    ) -> Result<Self, String>
    where
        F: Fn(GestureEvent) + Send + 'static,
    {
        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&is_running);

        let thread_handle = std::thread::Builder::new()
            .name("omniring-gesture-worker".to_string())
            .spawn(move || {
                let mut detector = GestureDetector::new(config, trigger_button);

                while running_clone.load(Ordering::Relaxed) {
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
        })
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

