//! Built-in application discovery extension.

#[path = "ApplicationArguments.rs"]
mod application_arguments;
#[path = "ApplicationConfig.rs"]
mod application_config;
#[path = "ApplicationDatabase.rs"]
mod application_database;
#[path = "ApplicationEntry.rs"]
mod application_entry;
#[path = "ApplicationError.rs"]
mod application_error;
#[path = "ApplicationIndex.rs"]
mod application_index;
mod candidate_selection;
#[path = "DiscoveryCommand.rs"]
mod discovery_command;
#[path = "DiscoveryServices.rs"]
mod discovery_services;
#[path = "DiscoveryState.rs"]
mod discovery_state;
#[path = "DiscoveryWorker.rs"]
mod discovery_worker;
#[path = "IconCache.rs"]
mod icon_cache;
#[path = "IconNormalizer.rs"]
mod icon_normalizer;
mod image_resize;
#[cfg(any(windows, test))]
mod legacy_icon;
mod migrations;
mod normalization;
mod platform;
#[path = "RuntimeEvent.rs"]
mod runtime_event;
#[path = "RuntimePaths.rs"]
mod runtime_paths;
#[path = "ScanReport.rs"]
mod scan_report;

pub use application_arguments::*;
pub use application_config::*;
pub use application_database::*;
pub use application_entry::*;
pub use application_error::*;
pub use application_index::*;
pub use candidate_selection::*;
pub(crate) use discovery_command::*;
pub(crate) use discovery_services::*;
pub(crate) use discovery_state::*;
pub use discovery_worker::*;
pub use icon_cache::*;
pub(crate) use icon_normalizer::*;
pub use runtime_event::*;
pub use runtime_paths::*;
pub use scan_report::*;

pub const EXTENSION_ID: &str = "com.nanika.application";
pub const RUN_ACTION_ID: &str = "application.run";

#[cfg(test)]
#[path = "../tests/ApplicationArguments.rs"]
mod application_arguments_tests;
#[cfg(test)]
#[path = "../tests/ApplicationConfig.rs"]
mod application_config_tests;
#[cfg(test)]
#[path = "../tests/ApplicationDatabase.rs"]
mod application_database_tests;
#[cfg(test)]
#[path = "../tests/ApplicationError.rs"]
mod application_error_tests;
#[cfg(test)]
#[path = "../tests/ApplicationIndex.rs"]
mod application_index_tests;
#[cfg(test)]
#[path = "../tests/candidate_selection.rs"]
mod candidate_selection_tests;
#[cfg(test)]
#[path = "../tests/IconCache.rs"]
mod icon_cache_tests;
#[cfg(test)]
#[path = "../tests/IconNormalizer.rs"]
mod icon_normalizer_tests;
#[cfg(test)]
#[path = "../tests/image_resize.rs"]
mod image_resize_tests;
#[cfg(test)]
#[path = "../tests/legacy_icon.rs"]
mod legacy_icon_tests;
