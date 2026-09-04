//! UI-independent host runtime and extension supervision boundaries.

#[path = "AcpConnectionContext.rs"]
mod acp_connection_context;
#[path = "AcpExtensionCommand.rs"]
mod acp_extension_command;
#[path = "AcpExtensionProcess.rs"]
mod acp_extension_process;
mod acp_transport;
#[path = "BoundedLogWriter.rs"]
mod bounded_log_writer;
#[path = "DiagnosticRecordKey.rs"]
mod diagnostic_record_key;
#[path = "DiagnosticSource.rs"]
mod diagnostic_source;
#[path = "Diagnostics.rs"]
mod diagnostics;
#[path = "DistributionExtension.rs"]
mod distribution_extension;
#[path = "DistributionInventory.rs"]
mod distribution_inventory;
#[path = "ExtensionCommand.rs"]
mod extension_command;
#[path = "ExtensionInvocation.rs"]
mod extension_invocation;
#[path = "ExtensionInvocationOutcome.rs"]
mod extension_invocation_outcome;
#[path = "ExtensionInvocationOutput.rs"]
mod extension_invocation_output;
#[path = "ExtensionInvocationOutputState.rs"]
mod extension_invocation_output_state;
#[path = "ExtensionInvocationResult.rs"]
mod extension_invocation_result;
#[path = "ExtensionLimits.rs"]
mod extension_limits;
#[path = "ExtensionNotifier.rs"]
mod extension_notifier;
#[path = "ExtensionProcess.rs"]
mod extension_process;
mod extension_process_command;
#[allow(unsafe_code)]
#[path = "ExtensionProcessTree.rs"]
mod extension_process_tree;
#[path = "ExtensionRefresh.rs"]
mod extension_refresh;
#[path = "ExtensionRuntime.rs"]
mod extension_runtime;
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
#[path = "ExtensionSettingsResult.rs"]
mod extension_settings_result;
#[path = "ExtensionSettingsUpdate.rs"]
mod extension_settings_update;
#[path = "ExtensionViewRequest.rs"]
mod extension_view_request;
#[path = "ExtensionViewRequestKind.rs"]
mod extension_view_request_kind;
#[path = "ExtensionViewUpdate.rs"]
mod extension_view_update;
#[path = "ExtensionViewUpdatePayload.rs"]
mod extension_view_update_payload;
#[path = "ExtensionWork.rs"]
mod extension_work;
#[path = "HostDiagnostic.rs"]
mod host_diagnostic;
#[path = "HostServiceHandler.rs"]
mod host_service_handler;
#[path = "HostServiceRouter.rs"]
mod host_service_router;
#[path = "RuntimeInvocationCompletion.rs"]
mod runtime_invocation_completion;
#[path = "RuntimeInvocationUpdate.rs"]
mod runtime_invocation_update;
#[path = "RuntimeOutputUpdate.rs"]
mod runtime_output_update;
#[path = "RuntimeService.rs"]
mod runtime_service;
#[path = "RuntimeSettingsUpdate.rs"]
mod runtime_settings_update;
#[path = "RuntimeUpdateBatch.rs"]
mod runtime_update_batch;
#[path = "RuntimeViewCompletion.rs"]
mod runtime_view_completion;
#[path = "RuntimeViewUpdate.rs"]
mod runtime_view_update;
#[path = "SupervisorError.rs"]
mod supervisor_error;

pub(crate) use acp_connection_context::*;
pub(crate) use acp_extension_command::*;
pub use acp_extension_process::*;
pub(crate) use acp_transport::*;
pub(crate) use bounded_log_writer::*;
pub(crate) use diagnostic_record_key::*;
pub(crate) use diagnostic_source::*;
pub use diagnostics::*;
pub use distribution_extension::*;
pub use distribution_inventory::*;
pub(crate) use extension_command::*;
pub(crate) use extension_invocation::*;
pub(crate) use extension_invocation_outcome::*;
pub(crate) use extension_invocation_output::*;
pub(crate) use extension_invocation_output_state::*;
pub(crate) use extension_invocation_result::*;
pub use extension_limits::*;
pub(crate) use extension_notifier::*;
pub use extension_process::*;
pub(crate) use extension_process_command::*;
pub(crate) use extension_process_tree::*;
pub(crate) use extension_refresh::*;
pub use extension_runtime::*;
pub use extension_runtime_invocation::*;
pub use extension_search_coordinator::*;
pub(crate) use extension_search_query::*;
pub(crate) use extension_search_state::*;
pub(crate) use extension_search_worker::*;
pub(crate) use extension_search_worker_context::*;
pub(crate) use extension_settings_result::*;
pub(crate) use extension_settings_update::*;
pub(crate) use extension_view_request::*;
pub(crate) use extension_view_request_kind::*;
pub(crate) use extension_view_update::*;
pub(crate) use extension_view_update_payload::*;
pub(crate) use extension_work::*;
pub use host_diagnostic::*;
pub use host_service_handler::*;
pub(crate) use host_service_router::*;
pub use runtime_invocation_completion::*;
pub use runtime_invocation_update::*;
pub use runtime_output_update::*;
pub use runtime_service::*;
pub use runtime_settings_update::*;
pub use runtime_update_batch::*;
pub use runtime_view_completion::*;
pub use runtime_view_update::*;
pub use supervisor_error::*;

pub use nanika_core::{DiagnosticCategory, DiagnosticCode};

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
            let icon_key = entry
                .icon
                .filter(nanika_protocol::IconReference::is_valid)
                .map(|icon| icon.key().to_owned());
            nanika_search::Candidate::new(
                extension_id,
                entry.entry_id,
                entry.title,
                entry.action_id,
                entry.aliases,
            )
            .with_subtitle(entry.subtitle)
            .with_icon_key(icon_key)
        })
        .collect();
    search.publish_extension_snapshot(extension_id, generation, candidates)
}

#[cfg(test)]
#[path = "../tests/acp_transport.rs"]
mod acp_transport_tests;
#[cfg(test)]
#[path = "../tests/BoundedLogWriter.rs"]
mod bounded_log_writer_tests;
#[cfg(test)]
#[path = "../tests/Diagnostics.rs"]
mod diagnostics_tests;
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
