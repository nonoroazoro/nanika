use super::{classify_registration, set_enabled_for, startup_command, status_for};
use crate::StartupStatus;
use windows_registry::CURRENT_USER;

const TEST_VALUE_NAME: &str = "NanikaStartupTest";

struct TestKey(String);

impl TestKey {
    fn new() -> Self {
        Self(format!(
            r"Software\Nanika\Tests\Startup\{}",
            std::process::id()
        ))
    }
}

impl Drop for TestKey {
    fn drop(&mut self) {
        let _ = CURRENT_USER.remove_tree(&self.0);
    }
}

#[test]
fn command_quotes_the_executable_and_starts_hidden() {
    let command = startup_command(std::path::Path::new(r"C:\Program Files\Nanika\nanika.exe"))
        .expect("command should be valid");
    assert_eq!(
        command,
        r#""C:\Program Files\Nanika\nanika.exe" --background"#
    );
}

#[test]
fn stale_registration_requires_repair() {
    assert_eq!(
        classify_registration(Some(r#""C:\Old\nanika.exe" --background"#), "expected"),
        StartupStatus::NeedsRepair
    );
    assert_eq!(
        classify_registration(None, "expected"),
        StartupStatus::Disabled
    );
}

#[test]
fn registry_round_trip_preserves_startup_semantics() {
    let key = TestKey::new();
    let executable = std::path::Path::new(r"C:\Program Files\Nanika\nanika.exe");

    assert_eq!(
        status_for(executable, &key.0, TEST_VALUE_NAME).expect("status should be readable"),
        StartupStatus::Disabled
    );
    assert_eq!(
        set_enabled_for(executable, true, &key.0, TEST_VALUE_NAME)
            .expect("registration should be enabled"),
        StartupStatus::Enabled
    );
    assert_eq!(
        status_for(executable, &key.0, TEST_VALUE_NAME).expect("status should be readable"),
        StartupStatus::Enabled
    );
    assert_eq!(
        set_enabled_for(executable, false, &key.0, TEST_VALUE_NAME)
            .expect("registration should be disabled"),
        StartupStatus::Disabled
    );
    assert_eq!(
        status_for(executable, &key.0, TEST_VALUE_NAME).expect("status should be readable"),
        StartupStatus::Disabled
    );
}
