use super::{classify_registration, startup_command};
use crate::StartupStatus;

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
