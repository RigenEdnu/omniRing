pub mod config;
pub mod watcher;

pub use config::{ActionConfig, ProfileConfig, ProfileManager, SliceItem, SubmenuConfig};
pub use watcher::{get_current_foreground_process, ForegroundWatcher, ProcessInfo};

