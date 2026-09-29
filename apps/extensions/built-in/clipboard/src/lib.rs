//! Built-in local clipboard history extension.

mod capture;
#[path = "ClipboardChange.rs"]
mod clipboard_change;
#[path = "ClipboardCommand.rs"]
mod clipboard_command;
#[path = "ClipboardConfig.rs"]
mod clipboard_config;
#[path = "ClipboardContentLease.rs"]
mod clipboard_content_lease;
#[path = "ClipboardDatabase.rs"]
mod clipboard_database;
#[path = "ClipboardEntry.rs"]
mod clipboard_entry;
#[path = "ClipboardItem.rs"]
mod clipboard_item;
#[path = "ClipboardMonitor.rs"]
mod clipboard_monitor;
#[path = "ClipboardPayloads.rs"]
mod clipboard_payloads;
#[path = "ClipboardPresentation.rs"]
mod clipboard_presentation;
#[path = "ClipboardQuery.rs"]
mod clipboard_query;
#[path = "ClipboardQueryEntry.rs"]
mod clipboard_query_entry;
#[path = "ClipboardStore.rs"]
mod clipboard_store;
#[path = "ClipboardViewState.rs"]
mod clipboard_view_state;
#[path = "ClipboardWatcherHandler.rs"]
mod clipboard_watcher_handler;
#[path = "ClipboardWorker.rs"]
mod clipboard_worker;
#[path = "EncodedClipboardContent.rs"]
mod encoded_clipboard_content;
#[path = "FileIconResolution.rs"]
mod file_icon_resolution;
#[path = "FileIconWorker.rs"]
mod file_icon_worker;
mod labels;
mod query;
#[path = "RuntimePaths.rs"]
mod runtime_paths;
mod view;

pub(crate) use capture::*;
pub use clipboard_change::*;
pub(crate) use clipboard_command::*;
pub use clipboard_config::*;
pub use clipboard_content_lease::*;
pub use clipboard_database::*;
pub use clipboard_entry::*;
pub(crate) use clipboard_item::*;
pub use clipboard_monitor::*;
pub(crate) use clipboard_payloads::*;
pub use clipboard_presentation::*;
pub(crate) use clipboard_query::*;
pub(crate) use clipboard_query_entry::*;
pub use clipboard_store::*;
pub use clipboard_view_state::*;
pub(crate) use clipboard_watcher_handler::*;
pub use clipboard_worker::*;
pub(crate) use encoded_clipboard_content::*;
pub(crate) use file_icon_resolution::*;
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

#[path = "ClipboardPreview.rs"]
mod clipboard_preview;
pub(crate) use clipboard_preview::ClipboardPreview;
#[path = "ClipboardWindow.rs"]
mod clipboard_window;
pub(crate) use clipboard_window::ClipboardWindow;
