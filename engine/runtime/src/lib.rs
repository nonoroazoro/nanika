//! UI-independent host runtime and extension supervision boundaries.

#[path = "OperationError.rs"]
mod operation_error;
pub use operation_error::OperationError;
#[path = "OperationErrorCode.rs"]
mod operation_error_code;
pub use operation_error_code::OperationErrorCode;
#[path = "RuntimeWarning.rs"]
mod runtime_warning;
pub use runtime_warning::RuntimeWarning;
#[path = "LifecycleFailure.rs"]
mod lifecycle_failure;
pub use lifecycle_failure::{LifecycleFailure, MAX_LIFECYCLE_DIAGNOSTIC_BYTES};
#[path = "LifecycleError.rs"]
mod lifecycle_error;
pub use lifecycle_error::LifecycleError;

#[path = "ExtensionResourceScope.rs"]
mod extension_resource_scope;
pub use extension_resource_scope::ExtensionResourceScope;

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
#[path = "ExtensionTextRead.rs"]
mod extension_text_read;
#[path = "TextReadReply.rs"]
mod text_read_reply;
pub(crate) use extension_text_read::ExtensionTextRead;
pub(crate) use text_read_reply::{TextReadCompletion, TextReadReply};
#[path = "ConfigurationSaveError.rs"]
mod configuration_save_error;
#[path = "ConfigurationSaveOutcome.rs"]
mod configuration_save_outcome;
pub use configuration_save_error::ConfigurationSaveError;
#[path = "ConfigurationSaveReceipt.rs"]
mod configuration_save_receipt;
pub(crate) use configuration_reply::{ConfigurationCompletion, ConfigurationReply};
pub use configuration_save_outcome::ConfigurationSaveOutcome;
pub use configuration_save_receipt::ConfigurationSaveReceipt;
pub use nanika_extension_package::MAX_CONFIGURATION_STRING_BYTES;
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
pub use extension_interruption::*;
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

/// Prepare the full publication before modifying the search owner.
pub(crate) fn search_candidates(
    extension_id: &str,
    entries: Vec<nanika_protocol::Candidate>,
    resources: Option<&nanika_localization::ResourceSet>
) -> Result<Vec<nanika_search::Candidate>, String> {
    let mut prepared = Vec::with_capacity(entries.len());
    let mut bytes = 0usize;
    for entry in entries {
        let search_text = nanika_localization::candidate_search_text(
            &entry,
            resources,
            nanika_platform::target_platform().as_str()
        )?;
        for value in &search_text {
            bytes = bytes
                .checked_add(value.len())
                .filter(|bytes| *bytes <= nanika_protocol::MAX_EXTENSION_MESSAGE_BYTES)
                .ok_or("prepared search publication exceeds its text budget")?;
        }
        prepared.push(
            nanika_search::Candidate::new(
                match entry.kind {
                    nanika_protocol::CandidateKind::Action => nanika_search::CandidateKind::Action,
                    nanika_protocol::CandidateKind::View => nanika_search::CandidateKind::View
                },
                extension_id,
                entry.entry_id,
                entry.title,
                entry.action_id,
                entry.actions,
                search_text
            )
            .with_subtitle(entry.subtitle)
            .with_icon(entry.icon.filter(nanika_protocol::IconSource::is_valid))
        );
    }
    Ok(prepared)
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

#[path = "ExtensionOperationGate.rs"]
mod extension_operation_gate;
pub(crate) use extension_operation_gate::*;

#[path = "ExtensionOperationReservation.rs"]
mod extension_operation_reservation;
pub(crate) use extension_operation_reservation::*;

#[path = "ExtensionInstance.rs"]
mod extension_instance;
pub(crate) use extension_instance::*;

#[path = "InstanceHostServices.rs"]
mod instance_host_services;
pub(crate) use instance_host_services::*;

#[path = "RuntimeExtensionState.rs"]
mod runtime_extension_state;
pub use runtime_extension_state::*;

#[path = "ExtensionConnectionExit.rs"]
mod extension_connection_exit;
pub(crate) use extension_connection_exit::ExtensionConnectionExit;

#[path = "RefreshReply.rs"]
mod refresh_reply;
pub(crate) use refresh_reply::{RefreshCompletion, RefreshReply};

#[path = "CatalogTransfer.rs"]
mod catalog_transfer;
use catalog_transfer::CatalogTransfer;

#[path = "CatalogBudget.rs"]
mod catalog_budget;
use catalog_budget::CatalogBudget;

#[path = "ExtensionInput.rs"]
mod extension_input;
use extension_input::ExtensionInput;

#[path = "PreparedHostService.rs"]
mod prepared_host_service;
pub use prepared_host_service::PreparedHostService;

#[path = "HostServiceReceipt.rs"]
mod host_service_receipt;
pub use host_service_receipt::HostServiceReceipt;

#[path = "ViewInvalidationNotifier.rs"]
mod view_invalidation_notifier;
use view_invalidation_notifier::ViewInvalidationNotifier;

#[path = "ConfigurationMenu.rs"]
mod configuration_menu;
pub use configuration_menu::ConfigurationMenu;

#[path = "ConfigurationMenuSection.rs"]
mod configuration_menu_section;
pub use configuration_menu_section::ConfigurationMenuSection;

#[path = "ConfigurationMenuControl.rs"]
mod configuration_menu_control;
pub use configuration_menu_control::ConfigurationMenuControl;

#[path = "PendingConfiguration.rs"]
mod pending_configuration;
use pending_configuration::PendingConfiguration;
#[path = "PendingRefresh.rs"]
mod pending_refresh;
use pending_refresh::PendingRefresh;
#[path = "PendingTextRead.rs"]
mod pending_text_read;
use pending_text_read::PendingTextRead;
