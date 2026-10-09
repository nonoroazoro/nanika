use nanika_protocol::{ClipboardContent, HostServiceRequest, LaunchArguments, LaunchDescriptor};

use super::{HostServiceHandler, HostServiceRouter, ProcessLauncher};

#[test]
fn reveal_requires_its_own_permission_before_accessing_the_platform() {
    let router = HostServiceRouter {
        system: Default::default(),
        launcher: Err("launcher unavailable".to_owned()),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Err("payload unavailable".to_owned()),
        permissions: Default::default()
    };
    router.register_permissions("com.nanika.test", ["process.launch".to_owned()]);
    let request = || HostServiceRequest::RevealPath {
        path: "relative.exe".to_owned()
    };
    let error = _prepare(&router, "com.nanika.test", request()).unwrap_err();
    assert!(error.contains("files.reveal"));
    router.register_permissions("com.nanika.test", ["files.reveal".to_owned()]);
    let error = _prepare(&router, "com.nanika.test", request()).unwrap_err();
    assert!(error.contains("launcher unavailable"));
}

#[test]
fn clipboard_and_payload_unavailability_do_not_disable_process_launch() {
    let router = HostServiceRouter {
        system: Default::default(),
        launcher: Ok(ProcessLauncher::spawn().expect("launcher")),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Err("payload unavailable".to_owned()),
        permissions: Default::default()
    };
    router.register_permissions("com.nanika.test", ["process.launch".to_owned()]);
    let result = _prepare(
        &router,
        "com.nanika.test",
        HostServiceRequest::Launch {
            descriptor: LaunchDescriptor::Program {
                program: String::new(),
                arguments: LaunchArguments::default(),
                working_directory: None
            }
        }
    )
    .expect("launch service should remain available")
    .admit()
    .unwrap()
    .recv_timeout(std::time::Duration::from_secs(1))
    .expect("launch service response");
    assert!(result.is_err());
}

#[test]
fn payload_roots_reject_invalid_extension_ids() {
    let router = HostServiceRouter {
        system: Default::default(),
        launcher: Err("launcher unavailable".to_owned()),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Ok(std::env::temp_dir()),
        permissions: Default::default()
    };
    let result = _prepare(
        &router,
        "../escape",
        HostServiceRequest::WriteClipboard {
            content: ClipboardContent::PngFile {
                path: "value.png".to_owned()
            }
        }
    );
    assert!(
        result
            .expect_err("invalid id should fail")
            .contains("invalid")
    );
}

#[test]
fn host_services_enforce_manifest_permissions() {
    let router = HostServiceRouter {
        system: Default::default(),
        launcher: Err("launcher unavailable".to_owned()),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Ok(std::env::temp_dir()),
        permissions: Default::default()
    };
    let error = _prepare(
        &router,
        "com.nanika.test",
        HostServiceRequest::WriteClipboard {
            content: ClipboardContent::Text {
                value: "value".to_owned()
            }
        }
    )
    .expect_err("missing permission should fail");
    assert!(error.contains("clipboard.write"));
}

#[test]
fn system_permissions_are_action_specific_and_denied_requests_do_not_start_a_worker() {
    use nanika_protocol::SystemAction;
    let router = HostServiceRouter {
        system: Default::default(),
        launcher: Err("unavailable".to_owned()),
        clipboard: Err("unavailable".to_owned()),
        payload_root: Err("unavailable".to_owned()),
        permissions: Default::default()
    };
    router.register_permissions("external.system", ["system.trash.open".to_owned()]);
    for action in [
        SystemAction::Lock,
        SystemAction::Sleep,
        SystemAction::TurnOffDisplays,
        SystemAction::LogOut,
        SystemAction::Restart,
        SystemAction::ShutDown,
        SystemAction::EmptyTrash
    ] {
        assert!(
            _prepare(
                &router,
                "external.system",
                HostServiceRequest::SystemAction { action }
            )
            .unwrap_err()
            .contains(action.permission())
        );
        assert!(router.system.get().is_none());
    }
    // Identical permission admission for a host-inventory identity.
    assert!(
        _prepare(
            &router,
            "com.nanika.system",
            HostServiceRequest::SystemAction {
                action: SystemAction::OpenTrash
            }
        )
        .is_err()
    );
    assert!(router.system.get().is_none());
}

fn _prepare<'a>(
    router: &'a HostServiceRouter,
    extension_id: &str,
    request: HostServiceRequest
) -> Result<crate::PreparedHostService<'a>, String> {
    router.prepare(extension_id, request, &mut || {
        crate::ExtensionInterruption::None
    })
}
