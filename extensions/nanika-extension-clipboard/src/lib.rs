//! Built-in local clipboard history extension.

mod capture;
#[path = "ClipboardCommand.rs"]
mod clipboard_command;
#[path = "ClipboardDatabase.rs"]
mod clipboard_database;
#[path = "ClipboardEntry.rs"]
mod clipboard_entry;
#[path = "ClipboardMonitor.rs"]
mod clipboard_monitor;
#[path = "ClipboardWatcherHandler.rs"]
mod clipboard_watcher_handler;
#[path = "ClipboardWorker.rs"]
mod clipboard_worker;
#[path = "EncodedClipboardContent.rs"]
mod encoded_clipboard_content;
mod migrations;
#[path = "RuntimePaths.rs"]
mod runtime_paths;

pub(crate) use capture::*;
pub(crate) use clipboard_command::*;
pub use clipboard_database::*;
pub use clipboard_entry::*;
pub use clipboard_monitor::*;
pub(crate) use clipboard_watcher_handler::*;
pub use clipboard_worker::*;
pub(crate) use encoded_clipboard_content::*;
pub use runtime_paths::*;

pub const EXTENSION_ID: &str = "com.nanika.clipboard";
pub const RESTORE_ACTION_ID: &str = "clipboard.restore";

#[cfg(test)]
#[path = "../tests/capture.rs"]
mod capture_tests;
#[cfg(test)]
#[path = "../tests/ClipboardDatabase.rs"]
mod clipboard_database_tests;
