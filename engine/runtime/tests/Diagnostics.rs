use std::ffi::OsStr;

use crate::diagnostics::maximum_level;

#[test]
fn verbose_diagnostics_are_opt_in() {
    assert_eq!(maximum_level(None), tracing::Level::INFO);
    assert_eq!(
        maximum_level(Some(OsStr::new("verbose"))),
        tracing::Level::DEBUG
    );
    assert_eq!(
        maximum_level(Some(OsStr::new("debug"))),
        tracing::Level::INFO
    );
}
