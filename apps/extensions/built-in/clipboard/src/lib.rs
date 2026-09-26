//! Built-in local clipboard history extension.

mod capture;
#[path = "ClipboardChange.rs"]
mod clipboard_change;
#[path = "ClipboardCommand.rs"]
mod clipboard_command;
#[path = "ClipboardConfig.rs"]
mod clipboard_config;
#[path = "ClipboardDatabase.rs"]
mod clipboard_database;
#[path = "ClipboardEntry.rs"]
mod clipboard_entry;
#[path = "ClipboardMonitor.rs"]
mod clipboard_monitor;
#[path = "ClipboardViewState.rs"]
mod clipboard_view_state;
#[path = "ClipboardWatcherHandler.rs"]
mod clipboard_watcher_handler;
#[path = "ClipboardWorker.rs"]
mod clipboard_worker;
#[path = "EncodedClipboardContent.rs"]
mod encoded_clipboard_content;
#[path = "FileIconWorker.rs"]
mod file_icon_worker;
#[path = "RuntimePaths.rs"]
mod runtime_paths;
mod view;

pub(crate) use capture::*;
pub use clipboard_change::*;
pub(crate) use clipboard_command::*;
pub use clipboard_config::*;
pub use clipboard_database::*;
pub use clipboard_entry::*;
pub use clipboard_monitor::*;
pub use clipboard_view_state::*;
pub(crate) use clipboard_watcher_handler::*;
pub use clipboard_worker::*;
pub(crate) use encoded_clipboard_content::*;
pub use file_icon_worker::*;
pub use runtime_paths::*;
pub use view::*;

pub const EXTENSION_ID: &str = "com.nanika.clipboard";
pub const COPY_ACTION_ID: &str = "clipboard.copy";
pub const CLEAR_ACTION_ID: &str = "clipboard.clear";
pub const VIEW_ID: &str = "clipboard.history";

#[cfg(test)]
#[path = "../tests/capture.rs"]
mod capture_tests;
#[cfg(test)]
#[path = "../tests/ClipboardConfig.rs"]
mod clipboard_config_tests;
#[cfg(test)]
#[path = "../tests/ClipboardDatabase.rs"]
mod clipboard_database_tests;

#[cfg(test)]
#[path = "../tests/view.rs"]
mod view_tests;
