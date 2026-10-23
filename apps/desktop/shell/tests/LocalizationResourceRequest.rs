use crate::{LocalizationResourceRequest, SearchSession, SettingsApplications};

#[test]
fn resource_reads_require_the_current_session_on_the_requested_surface() {
    let scope = nanika_host::ExtensionResourceScope {
        extension_id: "com.example.extension".into(),
        identity: "a".repeat(64)
    };
    let mut settings = SettingsApplications::default();
    let mut launcher = SearchSession::new(11, tauri::ipc::Channel::new(|_| Ok(())));
    let request = LocalizationResourceRequest::Settings {
        session_id: 0,
        scope: scope.clone()
    };
    assert!(
        request
            .authorize_session(Some(&launcher), &settings)
            .is_err()
    );
    settings.subscribe(12, tauri::ipc::Channel::new(|_| Ok(())));
    for session_id in [0, 11, 13] {
        let request = LocalizationResourceRequest::Settings {
            session_id,
            scope: scope.clone()
        };
        assert!(
            request
                .authorize_session(Some(&launcher), &settings)
                .is_err()
        );
    }
    let request = LocalizationResourceRequest::Settings {
        session_id: 12,
        scope: scope.clone()
    };
    assert!(request.authorize_session(None, &settings).is_ok());
    settings.subscribe(13, tauri::ipc::Channel::new(|_| Ok(())));
    assert!(
        request
            .authorize_session(Some(&launcher), &settings)
            .is_err()
    );
    let request = LocalizationResourceRequest::Launcher {
        session_id: 11,
        instance_id: 1,
        scope
    };
    assert_eq!(
        request.authorize_session(None, &settings).err(),
        Some(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::SessionUnavailable
        ))
    );
    assert!(
        request
            .authorize_session(Some(&launcher), &settings)
            .is_ok()
    );
    launcher.transport_error = Some("Transport failed".into());
    assert_eq!(
        request.authorize_session(Some(&launcher), &settings).err(),
        Some(nanika_host::OperationError::with_detail(
            nanika_host::OperationErrorCode::SessionUnavailable,
            "Transport failed"
        ))
    );
    launcher.transport_error = None;
    launcher.id = 14;
    assert!(
        request
            .authorize_session(Some(&launcher), &settings)
            .is_err()
    );
}

#[test]
fn resource_wire_contract_rejects_paths_extra_authority_and_missing_instance() {
    let settings = serde_json::json!({
        "surface":"settings", "sessionId":1,
        "scope":{"extensionId":"com.example.extension", "identity":"a".repeat(64)}
    });
    assert!(serde_json::from_value::<LocalizationResourceRequest>(settings.clone()).is_ok());
    for field in ["path", "namespace", "instanceId"] {
        let mut invalid = settings.clone();
        invalid[field] = serde_json::json!("unexpected");
        assert!(serde_json::from_value::<LocalizationResourceRequest>(invalid).is_err());
    }
    let mut launcher = settings;
    launcher["surface"] = serde_json::json!("launcher");
    assert!(serde_json::from_value::<LocalizationResourceRequest>(launcher.clone()).is_err());
    launcher["instanceId"] = serde_json::json!(2);
    assert!(serde_json::from_value::<LocalizationResourceRequest>(launcher.clone()).is_ok());
    launcher["scope"]["path"] = serde_json::json!("../outside.ftl");
    assert!(serde_json::from_value::<LocalizationResourceRequest>(launcher).is_err());
}
