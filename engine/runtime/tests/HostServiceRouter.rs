use nanika_protocol::{ClipboardContent, HostServiceRequest, LaunchArguments, LaunchDescriptor};

use super::{HostServiceHandler, HostServiceRouter, ProcessLauncher};

#[test]
fn clipboard_and_payload_unavailability_do_not_disable_process_launch() {
    let router = HostServiceRouter {
        launcher: Ok(ProcessLauncher::spawn().expect("launcher")),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Err("payload unavailable".to_owned()),
        permissions: Default::default(),
    };
    router.register_permissions("com.nanika.test", ["process.launch".to_owned()]);
    let result = router
        .submit(
            "com.nanika.test",
            HostServiceRequest::Launch {
                descriptor: LaunchDescriptor::Program {
                    program: String::new(),
                    arguments: LaunchArguments::default(),
                    working_directory: None,
                },
            },
        )
        .expect("launch service should remain available")
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("launch service response");
    assert!(result.is_err());
}

#[test]
fn payload_roots_reject_invalid_extension_ids() {
    let router = HostServiceRouter {
        launcher: Err("launcher unavailable".to_owned()),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Ok(std::env::temp_dir()),
        permissions: Default::default(),
    };
    let result = router.submit(
        "../escape",
        HostServiceRequest::WriteClipboard {
            content: ClipboardContent::PngFile {
                path: "value.png".to_owned(),
            },
        },
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
        launcher: Err("launcher unavailable".to_owned()),
        clipboard: Err("clipboard unavailable".to_owned()),
        payload_root: Ok(std::env::temp_dir()),
        permissions: Default::default(),
    };
    let error = router
        .submit(
            "com.nanika.test",
            HostServiceRequest::WriteClipboard {
                content: ClipboardContent::Text {
                    value: "value".to_owned(),
                },
            },
        )
        .expect_err("missing permission should fail");
    assert!(error.contains("clipboard.write"));
}
