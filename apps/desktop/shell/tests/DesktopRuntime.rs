use nanika_host::{OperationError, OperationErrorCode as Code};

use crate::{DesktopRuntime, SearchSession};

#[test]
fn startup_diagnostics_require_the_current_launcher_session() {
    let mut runtime = DesktopRuntime::default();
    runtime.fail_startup(OperationError::with_detail(
        Code::HostStorageUnavailable,
        "SQLITE_CANTOPEN: /Original/私密/nanika.db"
    ));
    assert!(matches!(
        runtime.startup_diagnostic(7),
        Err(OperationError::Host {
            code: Code::SessionUnavailable,
            ..
        })
    ));
    runtime.session = Some(SearchSession::new(7, tauri::ipc::Channel::new(|_| Ok(()))));
    assert!(matches!(
        runtime.startup_diagnostic(8),
        Err(OperationError::Host {
            code: Code::SessionUnavailable,
            ..
        })
    ));
    let failure = runtime.startup_diagnostic(7).unwrap();
    assert_eq!(
        failure.error,
        OperationError::new(Code::HostStorageUnavailable)
    );
    assert_eq!(
        failure.diagnostic.as_deref(),
        Some("SQLITE_CANTOPEN: /Original/私密/nanika.db")
    );
    assert_eq!(runtime.startup_diagnostic(7).unwrap(), failure);
    runtime.session = None;
    assert!(runtime.startup_diagnostic(7).is_err());
}

#[test]
fn a_healthy_runtime_does_not_invent_a_startup_diagnostic() {
    let runtime = DesktopRuntime {
        session: Some(SearchSession::new(1, tauri::ipc::Channel::new(|_| Ok(())))),
        ..Default::default()
    };
    assert!(matches!(
        runtime.startup_diagnostic(1),
        Err(OperationError::Host {
            code: Code::StartupDiagnosticUnavailable,
            ..
        })
    ));
}
