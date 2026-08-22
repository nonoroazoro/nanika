use nanika_platform::StartupStatus;

use crate::settings_view::startup_action;

#[test]
fn approval_opens_system_settings() {
    assert_eq!(
        startup_action(Some(StartupStatus::RequiresApproval)),
        Some(("Open System Settings", true))
    );
}

#[test]
fn missing_bundle_has_no_startup_action() {
    assert_eq!(startup_action(Some(StartupStatus::NotFound)), None);
}
