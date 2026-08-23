//! Host-owned UI and extension supervision boundaries.

#[path = "BuiltinExtensionSpec.rs"]
mod builtin_extension_spec;
mod builtins;
#[path = "Diagnostics.rs"]
mod diagnostics;
#[path = "ExtensionCommand.rs"]
mod extension_command;
#[path = "ExtensionInvocation.rs"]
mod extension_invocation;
#[path = "ExtensionInvocationResult.rs"]
mod extension_invocation_result;
#[path = "ExtensionLimits.rs"]
mod extension_limits;
#[path = "ExtensionNotifier.rs"]
mod extension_notifier;
#[path = "ExtensionProcess.rs"]
mod extension_process;
#[path = "ExtensionRefresh.rs"]
mod extension_refresh;
#[path = "ExtensionSearchCoordinator.rs"]
mod extension_search_coordinator;
#[path = "ExtensionSearchQuery.rs"]
mod extension_search_query;
#[path = "ExtensionSearchState.rs"]
mod extension_search_state;
#[path = "ExtensionSearchWorker.rs"]
mod extension_search_worker;
#[path = "ExtensionSettingsResult.rs"]
mod extension_settings_result;
#[path = "ExtensionSettingsUpdate.rs"]
mod extension_settings_update;
#[path = "ExtensionWork.rs"]
mod extension_work;
#[path = "HistoryDirection.rs"]
mod history_direction;
#[path = "HostApp.rs"]
mod host_app;
#[path = "HostConfig.rs"]
mod host_config;
#[path = "HostConfigCommand.rs"]
mod host_config_command;
#[path = "HostConfigService.rs"]
mod host_config_service;
#[path = "HostEvent.rs"]
mod host_event;
#[path = "HostRuntime.rs"]
mod host_runtime;
#[path = "HostServiceHandler.rs"]
mod host_service_handler;
#[path = "HostServiceRouter.rs"]
mod host_service_router;
#[path = "OverlayMotion.rs"]
mod overlay_motion;
#[path = "PendingExtension.rs"]
mod pending_extension;
#[path = "PendingHostSettings.rs"]
mod pending_host_settings;
mod render_preparation;
#[path = "SettingsAction.rs"]
mod settings_action;
#[path = "SettingsState.rs"]
mod settings_state;
mod settings_view;
#[path = "SupervisorError.rs"]
mod supervisor_error;

pub(crate) use builtin_extension_spec::*;
pub use diagnostics::*;
pub(crate) use extension_command::*;
pub(crate) use extension_invocation::*;
pub(crate) use extension_invocation_result::*;
pub use extension_limits::*;
pub(crate) use extension_notifier::*;
pub use extension_process::*;
pub(crate) use extension_refresh::*;
pub use extension_search_coordinator::*;
pub(crate) use extension_search_query::*;
pub(crate) use extension_search_state::*;
pub(crate) use extension_search_worker::*;
pub(crate) use extension_settings_result::*;
pub(crate) use extension_settings_update::*;
pub(crate) use extension_work::*;
pub(crate) use history_direction::*;
pub use host_app::HostApp;
pub(crate) use host_config::*;
pub(crate) use host_config_command::*;
pub(crate) use host_config_service::*;
pub(crate) use host_event::*;
pub(crate) use host_runtime::*;
pub use host_service_handler::*;
pub(crate) use host_service_router::*;
pub(crate) use overlay_motion::*;
pub(crate) use pending_extension::*;
pub(crate) use pending_host_settings::*;
pub use render_preparation::*;
pub(crate) use settings_action::*;
pub(crate) use settings_state::*;
pub use supervisor_error::*;

/// Publish one protocol snapshot into the shared search owner.
pub fn publish_extension_snapshot(
    search: &nanika_search::SearchHandle,
    extension_id: &str,
    generation: u64,
    entries: Vec<nanika_protocol::Candidate>,
) -> Result<(), nanika_search::SearchQueueError> {
    let candidates = entries
        .into_iter()
        .map(|entry| {
            nanika_search::Candidate::new(
                extension_id,
                entry.entry_id,
                entry.title,
                entry.action_id,
                entry.aliases,
            )
        })
        .collect();
    search.publish_extension_snapshot(extension_id, generation, candidates)
}

#[cfg(test)]
#[path = "../tests/HostApp.rs"]
mod host_app_tests;
#[cfg(test)]
#[path = "../tests/HostConfigService.rs"]
mod host_config_service_tests;
#[cfg(test)]
#[path = "../tests/OverlayMotion.rs"]
mod overlay_motion_tests;
#[cfg(test)]
#[path = "../tests/render_preparation.rs"]
mod render_preparation_tests;
#[cfg(test)]
#[path = "../tests/SettingsState.rs"]
mod settings_state_tests;
#[cfg(test)]
#[path = "../tests/settings_view.rs"]
mod settings_view_tests;
