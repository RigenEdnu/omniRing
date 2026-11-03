//! Dedicated foreground window watcher using Win32 WinEvent hook (EVENT_SYSTEM_FOREGROUND).
//! Resolves active foreground window HWND to process ID and executable image name via
//! QueryFullProcessImageNameW.

use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, PostThreadMessageW, TranslateMessage, MSG, WM_QUIT,
};

/// Win32 WinEvent hook constants.
pub const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
pub const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
pub const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

/// Process information describing the current foreground application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub process_name: String,
    pub executable_path: String,
    pub window_title: Option<String>,
}

/// Helper function to retrieve process ID, image path, and title from an active window handle.
pub fn get_process_info_from_hwnd(hwnd: HWND) -> Option<ProcessInfo> {
    if hwnd.0 == 0 {
        return None;
    }

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid == 0 {
        return None;
    }

    // Optional window title extraction
    let title = unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len > 0 {
            let mut buf = vec![0u16; (len + 1) as usize];
            let copied = GetWindowTextW(hwnd, &mut buf);
            if copied > 0 {
                Some(String::from_utf16_lossy(&buf[..copied as usize]))
            } else {
                None
            }
        } else {
            None
        }
    };

    // Open target process with minimal query permissions
    let (full_path, proc_name) = match unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
    } {
        Ok(h_proc) => {
            let mut buffer = vec![0u16; 1024];
            let mut size = buffer.len() as u32;
            let queried = unsafe {
                QueryFullProcessImageNameW(
                    h_proc,
                    PROCESS_NAME_WIN32,
                    PWSTR(buffer.as_mut_ptr()),
                    &mut size,
                )
            };
            unsafe {
                let _ = CloseHandle(h_proc);
            }

            if queried.is_ok() && size > 0 {
                let p = String::from_utf16_lossy(&buffer[..size as usize]);
                let name = Path::new(&p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("process_{}.exe", pid));
                (p, name)
            } else {
                (String::new(), format!("process_{}.exe", pid))
            }
        }
        Err(_) => (String::new(), format!("process_{}.exe", pid)),
    };

    Some(ProcessInfo {
        pid,
        process_name: proc_name,
        executable_path: full_path,
        window_title: title,
    })
}

/// Retrieves the process information of the current foreground window.
pub fn get_current_foreground_process() -> Option<ProcessInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0 == 0 {
        return None;
    }
    get_process_info_from_hwnd(hwnd)
}

struct WatcherGlobalState {
    sender: Sender<ProcessInfo>,
    last_pid: AtomicU32,
    own_pid: u32,
}

static WATCHER_STATE: RwLock<Option<WatcherGlobalState>> = RwLock::new(None);

/// Global WinEvent hook callback called by Windows on foreground window changes.
unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND || id_object != 0 || id_child != 0 {
        return;
    }

    let target_hwnd = if hwnd.0 != 0 {
        hwnd
    } else {
        GetForegroundWindow()
    };

    if target_hwnd.0 == 0 {
        return;
    }

    if let Some(info) = get_process_info_from_hwnd(target_hwnd) {
        if let Ok(guard) = WATCHER_STATE.read() {
            if let Some(state) = guard.as_ref() {
                // Ignore events originating from our own omniRing overlay / host process
                if info.pid == state.own_pid {
                    return;
                }

                // Deduplicate immediate repeat events on the same process
                let previous = state.last_pid.swap(info.pid, Ordering::Relaxed);
                if previous == info.pid {
                    return;
                }

                let _ = state.sender.try_send(info);
            }
        }
    }
}

/// Dedicated background worker managing the Win32 WinEvent foreground hook.
pub struct ForegroundWatcher {
    thread_handle: Option<JoinHandle<()>>,
    thread_id: u32,
    is_running: Arc<AtomicBool>,
    receiver: Receiver<ProcessInfo>,
}

impl ForegroundWatcher {
    /// Starts the foreground watcher worker on an isolated thread with a message loop.
    pub fn start() -> Result<Self, String> {
        let (tx, rx) = crossbeam_channel::bounded(64);
        let own_pid = unsafe { GetCurrentProcessId() };

        if let Ok(mut guard) = WATCHER_STATE.write() {
            *guard = Some(WatcherGlobalState {
                sender: tx,
                last_pid: AtomicU32::new(0),
                own_pid,
            });
        }

        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&is_running);
        let (init_tx, init_rx) = crossbeam_channel::bounded(1);

        let thread_handle = std::thread::Builder::new()
            .name("omniring-foreground-watcher".to_string())
            .spawn(move || {
                let thread_id = unsafe { GetCurrentThreadId() };

                let hook = unsafe {
                    SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_FOREGROUND,
                        None,
                        Some(win_event_proc),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    )
                };

                if hook.0.is_null() {
                    let _ = init_tx.send(Err("SetWinEventHook returned null handle".to_string()));
                    return;
                }

                let _ = init_tx.send(Ok(thread_id));

                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                // Cleanup hook on termination
                unsafe {
                    let _ = UnhookWinEvent(hook);
                }
                running_clone.store(false, Ordering::Relaxed);
            })
            .map_err(|e| format!("Failed to spawn foreground watcher thread: {}", e))?;

        let thread_id = match init_rx.recv() {
            Ok(Ok(id)) => id,
            Ok(Err(err)) => return Err(err),
            Err(e) => return Err(format!("Initialization channel failed: {}", e)),
        };

        Ok(Self {
            thread_handle: Some(thread_handle),
            thread_id,
            is_running,
            receiver: rx,
        })
    }

    /// Accessor for the receiver channel delivering foreground process change notifications.
    pub fn receiver(&self) -> &Receiver<ProcessInfo> {
        &self.receiver
    }

    /// Indicates whether the watcher thread is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Signals the watcher thread to stop and unhooks the WinEvent listener.
    pub fn stop(&mut self) {
        if self.is_running.swap(false, Ordering::Relaxed) {
            if self.thread_id != 0 {
                unsafe {
                    let _ = PostThreadMessageW(
                        self.thread_id,
                        WM_QUIT,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
            if let Some(handle) = self.thread_handle.take() {
                let _ = handle.join();
            }
            if let Ok(mut guard) = WATCHER_STATE.write() {
                *guard = None;
            }
        }
    }
}

impl Drop for ForegroundWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_info_serialization() {
        let info = ProcessInfo {
            pid: 1234,
            process_name: "code.exe".to_string(),
            executable_path: r"C:\Program Files\VSCode\code.exe".to_string(),
            window_title: Some("omniRing - Visual Studio Code".to_string()),
        };

        let json = serde_json::to_string(&info).expect("Failed to serialize ProcessInfo");
        let deserialized: ProcessInfo =
            serde_json::from_str(&json).expect("Failed to deserialize ProcessInfo");
        assert_eq!(info, deserialized);
    }
}

