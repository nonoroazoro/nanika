use std::error::Error;

use crate::ApplicationError;

#[test]
fn wrapped_application_errors_preserve_their_source() {
    let error = ApplicationError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));

    assert_eq!(
        error.source().map(ToString::to_string).as_deref(),
        Some("denied")
    );
}
