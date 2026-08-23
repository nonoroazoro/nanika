//! Host-owned paths and SQLite storage.

#![forbid(unsafe_code)]

#[path = "ExtensionDatabase.rs"]
mod extension_database;
mod extension_id;
#[path = "ExtensionKind.rs"]
mod extension_kind;
#[path = "HostDatabase.rs"]
mod host_database;
mod migrations;
#[path = "NanikaPaths.rs"]
mod nanika_paths;
#[path = "SearchStorageCommand.rs"]
mod search_storage_command;
#[path = "SearchStorageState.rs"]
mod search_storage_state;
#[path = "SearchStorageWorker.rs"]
mod search_storage_worker;
#[path = "StorageQueueError.rs"]
mod storage_queue_error;
#[path = "StoredExtension.rs"]
mod stored_extension;
#[path = "StoredExtensionLoad.rs"]
mod stored_extension_load;
#[path = "StoredUsage.rs"]
mod stored_usage;
mod time;

pub use extension_database::*;
pub use extension_id::*;
pub use extension_kind::*;
pub use host_database::*;
pub use nanika_paths::*;
pub(crate) use search_storage_command::*;
pub use search_storage_state::*;
pub use search_storage_worker::*;
pub use storage_queue_error::*;
pub use stored_extension::*;
pub use stored_extension_load::*;
pub use stored_usage::*;
pub(crate) use time::*;

#[cfg(test)]
#[path = "../tests/ExtensionDatabase.rs"]
mod extension_database_tests;
#[cfg(test)]
#[path = "../tests/HostDatabase.rs"]
mod host_database_tests;
#[cfg(test)]
#[path = "../tests/NanikaPaths.rs"]
mod nanika_paths_tests;
#[cfg(test)]
#[path = "../tests/SearchStorageWorker.rs"]
mod search_storage_worker_tests;
