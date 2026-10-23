use ts_rs::{Config, TS};

#[path = "../tests/bindings.rs"]
mod contracts;

/// Explicit tooling entry point. Ordinary tests never rewrite frontend sources.
#[test]
#[ignore = "run through bun run types:update or just check"]
fn export() {
    let output =
        std::env::var_os("NANIKA_TYPES_OUTPUT").expect("tooling must supply an output directory");
    // Match JSON numbers used by the existing IPC contract, not JavaScript bigint.
    let config = Config::new().with_large_int("number").with_out_dir(output);
    nanika_localization::PreparedResources::export_all(&config).expect("export PreparedResources");
    nanika_protocol::LocalizedText::export_all(&config).expect("export LocalizedText");
    nanika_localization::SupportedLocale::export_all(&config).expect("export SupportedLocale");
    nanika_localization::LanguagePreference::export_all(&config)
        .expect("export LanguagePreference");
    nanika_localization::LocalizationResources::export_all(&config)
        .expect("export LocalizationResources");
    nanika_localization::MessageContract::export_all(&config).expect("export MessageContract");
    std::fs::write(config.out_dir().join("localization-limits.ts"), format!(
        "// Generated from Rust localization bounds.\nexport const MAX_LIFECYCLE_DIAGNOSTIC_BYTES = {};\nexport const MAX_ERROR_MESSAGE_CHARS = {};\nexport const MAX_ERROR_DETAIL_BYTES = {};\nexport const MAX_MESSAGE_ID_BYTES = {};\nexport const MAX_MESSAGE_ARGUMENTS = {};\nexport const MAX_MESSAGE_ARGUMENT_BYTES = {};\nexport const MAX_EXPANDED_BYTES = {};\nexport const MAX_RESOURCE_BYTES = {};\nexport const MAX_LOCALE_BYTES = {};\nexport const MAX_LOCALE_RESOURCES = {};\nexport const MAX_CACHED_RESOURCE_SETS = {};\nexport const MAX_RESOURCE_LOADS = {};\n",
        nanika_host::MAX_LIFECYCLE_DIAGNOSTIC_BYTES,
        nanika_protocol::MAX_ERROR_MESSAGE_CHARS,
        nanika_protocol::MAX_ERROR_DETAIL_BYTES,
        nanika_protocol::MAX_MESSAGE_ID_BYTES,
        nanika_protocol::MAX_MESSAGE_ARGUMENTS,
        nanika_protocol::MAX_MESSAGE_ARGUMENT_BYTES,
        nanika_localization::MAX_EXPANDED_BYTES,
        nanika_localization::MAX_RESOURCE_BYTES,
        nanika_localization::MAX_LOCALE_BYTES,
        nanika_localization::MAX_LOCALE_RESOURCES,
        nanika_localization::MAX_CACHED_RESOURCE_SETS,
        nanika_localization::MAX_RESOURCE_LOADS
    )).expect("export localization limits");
    std::fs::write(config.out_dir().join("host-resources.ts"), format!(
        "// Generated from the validated host FTL resources.\nimport type {{ PreparedResources }} from \"./PreparedResources\";\nexport const hostResources: PreparedResources = {};\n",
        serde_json::to_string(&crate::host_resources::resources().expect("validate host FTL").prepared()).expect("serialize host resources")
    )).expect("export host resources");
    std::fs::write(
        config.out_dir().join("LocalizedMessage.ts"),
        nanika_localization::typescript_message_type(
            crate::host_resources::resources().expect("validate host FTL")
        )
    )
    .expect("export host message types");
    crate::ApplicationSnapshot::export_all(&config).expect("export ApplicationSnapshot");
    crate::RootSearchSnapshot::export_all(&config).expect("export RootSearchSnapshot");
    crate::PublishQueryRequest::export_all(&config).expect("export PublishQueryRequest");
    crate::InvokeCandidateRequest::export_all(&config).expect("export InvokeCandidateRequest");
    crate::ReadResultsRequest::export_all(&config).expect("export ReadResultsRequest");
    crate::ReadTextRequest::export_all(&config).expect("export ReadTextRequest");
    nanika_protocol::TextChunk::export_all(&config).expect("export TextChunk");
    nanika_host::OperationError::export_all(&config).expect("export OperationError");
    crate::ConfigurationMenuRequest::export_all(&config).expect("export ConfigurationMenuRequest");
    crate::LocalizationResourceRequest::export_all(&config)
        .expect("export LocalizationResourceRequest");
    crate::ConfigurationMenuOperation::export_all(&config)
        .expect("export ConfigurationMenuOperation");
    nanika_host::ConfigurationMenu::export_all(&config).expect("export ConfigurationMenu");
    crate::ContextMenuRequest::export_all(&config).expect("export ContextMenuRequest");
    crate::ViewEventRequest::export_all(&config).expect("export ViewEventRequest");
    crate::ViewEventReceipt::export_all(&config).expect("export ViewEventReceipt");
    crate::SettingsSnapshot::export_all(&config).expect("export SettingsSnapshot");
    crate::SettingsEvent::export_all(&config).expect("export SettingsEvent");
    crate::SettingsActionRequest::export_all(&config).expect("export SettingsActionRequest");
    crate::SaveSettingsRequest::export_all(&config).expect("export SaveSettingsRequest");
    crate::SettingsWindowAction::export_all(&config).expect("export SettingsWindowAction");
    crate::SettingsSearchEntry::export_all(&config).expect("export SettingsSearchEntry");
    crate::HostSettingsChange::export_all(&config).expect("export HostSettingsChange");
    nanika_platform::StartupStatus::export_all(&config).expect("export StartupStatus");
    nanika_host::ConfigurationSaveOutcome::export_all(&config)
        .expect("export ConfigurationSaveOutcome");
    crate::SettingsWriteResult::<nanika_config::LauncherPreferences>::export_all(&config)
        .expect("export SettingsWriteResult");
    std::fs::write(config.out_dir().join("limits.ts"), format!(
        "// Generated from Rust boundary constants.\nexport const DETAIL_TEXT_BATCH_CHARS = {};\nexport const MAX_DETAIL_TEXT_CHUNKS = {};\nexport const MAX_CONFIGURATION_STRING_BYTES = {};\nexport const MAX_VIEW_SEARCH_CHARS = {};\n",
        nanika_protocol::DETAIL_TEXT_BATCH_CHARS,
        nanika_protocol::MAX_DETAIL_TEXT_CHUNKS,
        nanika_host::MAX_CONFIGURATION_STRING_BYTES,
        nanika_protocol::MAX_VIEW_SEARCH_CHARS
    )).expect("export boundary limits");
    contracts::write(
        &config.out_dir().parent().unwrap().join("contracts.ts"),
        &config
    );
}
