use ts_rs::{Config, TS};

#[path = "../tests/bindings.rs"]
mod contracts;

/// Explicit tooling entry point. Ordinary tests never rewrite frontend sources.
#[test]
#[ignore = "run through bun run types:update or types:check"]
fn export() {
    let output =
        std::env::var_os("NANIKA_TYPES_OUTPUT").expect("tooling must supply an output directory");
    // Match JSON numbers used by the existing IPC contract, not JavaScript bigint.
    let config = Config::new().with_large_int("number").with_out_dir(output);
    crate::ApplicationSnapshot::export_all(&config).expect("export ApplicationSnapshot");
    crate::RootSearchSnapshot::export_all(&config).expect("export RootSearchSnapshot");
    crate::PublishQueryRequest::export_all(&config).expect("export PublishQueryRequest");
    crate::InvokeCandidateRequest::export_all(&config).expect("export InvokeCandidateRequest");
    crate::ReadResultsRequest::export_all(&config).expect("export ReadResultsRequest");
    crate::ContextMenuRequest::export_all(&config).expect("export ContextMenuRequest");
    crate::ViewEventRequest::export_all(&config).expect("export ViewEventRequest");
    crate::ViewEventReceipt::export_all(&config).expect("export ViewEventReceipt");
    crate::SettingsSnapshot::export_all(&config).expect("export SettingsSnapshot");
    crate::SettingsEvent::export_all(&config).expect("export SettingsEvent");
    crate::SaveSettingsRequest::export_all(&config).expect("export SaveSettingsRequest");
    crate::SettingsWindowAction::export_all(&config).expect("export SettingsWindowAction");
    crate::SettingsSearchEntry::export_all(&config).expect("export SettingsSearchEntry");
    crate::HostSettingsChange::export_all(&config).expect("export HostSettingsChange");
    nanika_platform::StartupStatus::export_all(&config).expect("export StartupStatus");
    nanika_host::ConfigurationSaveOutcome::export_all(&config)
        .expect("export ConfigurationSaveOutcome");
    crate::SettingsWriteResult::<nanika_config::LauncherPreferences>::export_all(&config)
        .expect("export SettingsWriteResult");
    contracts::write(
        &config.out_dir().parent().unwrap().join("contracts.ts"),
        &config,
    );
}
