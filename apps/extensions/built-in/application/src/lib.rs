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
#[path = "ApplicationSources.rs"]
mod application_sources;
#[path = "DiscoveryCommand.rs"]
mod discovery_command;
#[path = "DiscoveryServices.rs"]
mod discovery_services;
#[path = "DiscoveryState.rs"]
mod discovery_state;
#[path = "DiscoveryWorker.rs"]
mod discovery_worker;
#[path = "EntryPriority.rs"]
mod entry_priority;
pub(crate) use entry_priority::EntryPriority;
#[path = "IconCache.rs"]
mod icon_cache;
mod normalization;
mod platform;
#[path = "RuntimeEvent.rs"]
mod runtime_event;
#[path = "RuntimePaths.rs"]
mod runtime_paths;
#[path = "ScanCoverage.rs"]
mod scan_coverage;
#[path = "ScanReport.rs"]
mod scan_report;

pub use application_arguments::*;
pub use application_config::*;
pub use application_database::*;
pub use application_entry::*;
pub use application_error::*;
pub use application_index::*;
pub(crate) use discovery_command::*;
pub(crate) use discovery_services::*;
pub(crate) use discovery_state::*;
pub use discovery_worker::*;
pub use icon_cache::*;
pub(crate) use nanika_platform::normalize_icon_rgba;
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
#[path = "../tests/ApplicationEntry.rs"]
mod application_entry_tests;
#[cfg(test)]
#[path = "../tests/ApplicationError.rs"]
mod application_error_tests;
#[cfg(test)]
#[path = "../tests/IconCache.rs"]
mod icon_cache_tests;
#[cfg(test)]
#[path = "../tests/ScanCoverage.rs"]
mod scan_coverage_tests;

#[path = "ApplicationEntryData.rs"]
mod entry_data;
pub use entry_data::ApplicationEntryData;
