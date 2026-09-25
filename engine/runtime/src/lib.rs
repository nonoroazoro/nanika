//! UI-independent host runtime and extension supervision boundaries.

#[path = "AcpConnectionContext.rs"]
mod acp_connection_context;
#[path = "AcpExtensionCommand.rs"]
mod acp_extension_command;
#[path = "AcpExtensionProcess.rs"]
mod acp_extension_process;
mod acp_transport;
#[path = "BuiltInExtension.rs"]
mod built_in_extension;
#[path = "BuiltInExtensionInventory.rs"]
mod built_in_extension_inventory;
#[path = "ConfigurationReply.rs"]
mod configuration_reply;
#[path = "ConfigurationSaveOutcome.rs"]
mod configuration_save_outcome;
#[path = "ConfigurationSaveReceipt.rs"]
mod configuration_save_receipt;
pub(crate) use configuration_reply::{ConfigurationCompletion, ConfigurationReply};
pub use configuration_save_outcome::ConfigurationSaveOutcome;
pub use configuration_save_receipt::ConfigurationSaveReceipt;
#[path = "DiagnosticSource.rs"]
mod diagnostic_source;
#[path = "Diagnostics.rs"]
mod diagnostics;
#[path = "ExtensionCommand.rs"]
mod extension_command;
#[path = "ExtensionConfigurationRegistry.rs"]
mod extension_configuration_registry;
#[path = "ExtensionConfigurationUpdate.rs"]
mod extension_configuration_update;
#[path = "ExtensionInterruption.rs"]
mod extension_interruption;
#[path = "ExtensionInvocation.rs"]
mod extension_invocation;
#[path = "ExtensionInvocationOutcome.rs"]
mod extension_invocation_outcome;
#[path = "ExtensionInvocationOutput.rs"]
mod extension_invocation_output;
#[path = "ExtensionInvocationOutputState.rs"]
mod extension_invocation_output_state;
#[path = "ExtensionLimits.rs"]
mod extension_limits;
#[path = "ExtensionNotifier.rs"]
mod extension_notifier;
#[path = "ExtensionProcess.rs"]
mod extension_process;
#[path = "ExtensionRefresh.rs"]
mod extension_refresh;
#[path = "ExtensionRuntime.rs"]
mod extension_runtime;
#[path = "ExtensionRuntimeSource.rs"]
mod extension_runtime_source;
pub use extension_runtime_source::*;
#[path = "ExtensionRuntimeInvocation.rs"]
mod extension_runtime_invocation;
#[path = "ExtensionSearchCoordinator.rs"]
mod extension_search_coordinator;
#[path = "ExtensionSearchQuery.rs"]
mod extension_search_query;
#[path = "ExtensionSearchState.rs"]
mod extension_search_state;
#[path = "ExtensionSearchWorker.rs"]
mod extension_search_worker;
#[path = "ExtensionSearchWorkerContext.rs"]
mod extension_search_worker_context;
#[path = "ExtensionViewRequest.rs"]
mod extension_view_request;
#[path = "ExtensionViewRequestKind.rs"]
mod extension_view_request_kind;
#[path = "ExtensionWork.rs"]
mod extension_work;
#[path = "HostDiagnostic.rs"]
mod host_diagnostic;
#[path = "HostServiceHandler.rs"]
mod host_service_handler;
#[path = "HostServiceRouter.rs"]
mod host_service_router;
#[path = "RuntimeExtensionConfiguration.rs"]
mod runtime_extension_configuration;
#[path = "RuntimeExtensionInfo.rs"]
mod runtime_extension_info;
#[path = "RuntimeInvocationCompletion.rs"]
mod runtime_invocation_completion;
#[path = "RuntimeOutputUpdate.rs"]
mod runtime_output_update;
#[path = "RuntimeService.rs"]
mod runtime_service;
#[path = "RuntimeUpdateBatch.rs"]
mod runtime_update_batch;
#[path = "RuntimeViewCompletion.rs"]
mod runtime_view_completion;
#[path = "RuntimeViewInvalidation.rs"]
mod runtime_view_invalidation;
#[path = "SupervisorError.rs"]
mod supervisor_error;

pub(crate) use acp_connection_context::*;
pub(crate) use acp_extension_command::*;
pub use acp_extension_process::*;
pub(crate) use acp_transport::*;
pub use built_in_extension::*;
pub use built_in_extension_inventory::*;
pub(crate) use diagnostic_source::*;
pub use diagnostics::*;
pub(crate) use extension_command::*;
pub(crate) use extension_configuration_registry::*;
pub(crate) use extension_configuration_update::*;
pub(crate) use extension_interruption::*;
pub(crate) use extension_invocation::*;
pub use extension_invocation_outcome::*;
pub(crate) use extension_invocation_output::*;
pub(crate) use extension_invocation_output_state::*;
pub use extension_limits::*;
pub(crate) use extension_notifier::*;
pub use extension_process::*;
pub(crate) use extension_refresh::*;
pub use extension_runtime::*;
pub use extension_runtime_invocation::*;
pub use extension_search_coordinator::*;
pub(crate) use extension_search_query::*;
pub(crate) use extension_search_state::*;
pub(crate) use extension_search_worker::*;
pub(crate) use extension_search_worker_context::*;
pub(crate) use extension_view_request::*;
pub(crate) use extension_view_request_kind::*;
pub(crate) use extension_work::*;
pub use host_diagnostic::*;
pub use host_service_handler::*;
pub(crate) use host_service_router::*;
pub use runtime_extension_configuration::*;
pub use runtime_extension_info::*;
pub use runtime_invocation_completion::*;
pub use runtime_output_update::*;
pub use runtime_service::*;
pub use runtime_update_batch::*;
pub use runtime_view_completion::*;
pub use runtime_view_invalidation::*;
pub use supervisor_error::*;

pub use nanika_foundation::{DiagnosticCategory, DiagnosticCode};

/// Publish one protocol snapshot into the shared search owner.
pub fn publish_extension_snapshot(
    search: &nanika_search::SearchHandle,
    extension_id: &str,
    generation: u64,
    entries: Vec<nanika_protocol::Candidate>,
) -> Result<(), nanika_search::SearchQueueError> {
    tracing::debug!(
        extension_id,
        generation,
        candidates = entries.len(),
        "extension search snapshot received"
    );
    search.publish_extension_snapshot(
        extension_id,
        generation,
        search_candidates(extension_id, entries),
    )
}

pub(crate) fn search_candidates(
    extension_id: &str,
    entries: Vec<nanika_protocol::Candidate>,
) -> Vec<nanika_search::Candidate> {
    entries
        .into_iter()
        .map(|entry| {
            let icon_key = entry
                .icon
                .filter(nanika_protocol::IconReference::is_valid)
                .map(|icon| icon.key().to_owned());
            let contribution_icon = if icon_key.is_none() {
                entry.contribution_icon.map(|icon| icon.as_str().to_owned())
            } else {
                None
            };
            nanika_search::Candidate::new(
                match entry.kind {
                    nanika_protocol::CandidateKind::Action => nanika_search::CandidateKind::Action,
                    nanika_protocol::CandidateKind::View => nanika_search::CandidateKind::View,
                },
                extension_id,
                entry.entry_id,
                entry.title,
                entry.action_id,
                entry.actions,
                entry.aliases,
            )
            .with_subtitle(entry.subtitle)
            .with_icon_key(icon_key)
            .with_contribution_icon(contribution_icon)
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/acp_transport.rs"]
mod acp_transport_tests;
#[cfg(test)]
#[path = "../tests/BuiltInExtensionInventory.rs"]
mod built_in_extension_inventory_tests;
#[cfg(test)]
#[path = "../tests/Diagnostics.rs"]
mod diagnostics_tests;
#[cfg(test)]
#[path = "../tests/ExtensionConfigurationRegistry.rs"]
mod extension_configuration_registry_tests;
#[cfg(test)]
#[path = "../tests/ExtensionInvocationOutputState.rs"]
mod extension_invocation_output_state_tests;
#[cfg(test)]
#[path = "../tests/ExtensionRuntime.rs"]
mod extension_runtime_tests;
#[cfg(test)]
#[path = "../tests/ExtensionSearchWorker.rs"]
mod extension_search_worker_tests;
#[cfg(test)]
#[path = "../tests/HostDiagnostic.rs"]
mod host_diagnostic_tests;

#[path = "ExtensionWorkerLifetime.rs"]
mod extension_worker_lifetime;
use extension_worker_lifetime::ExtensionWorkerLifetime;

#[path = "ConfigurationOperation.rs"]
mod configuration_operation;
pub(crate) use configuration_operation::*;
#[path = "ConfigurationApplication.rs"]
mod configuration_application;
pub(crate) use configuration_application::*;

#[path = "ConfigurationProgressHandler.rs"]
mod configuration_progress_handler;
pub use configuration_progress_handler::*;
