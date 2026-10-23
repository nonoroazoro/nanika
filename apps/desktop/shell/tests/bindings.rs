use std::path::Path;

use serde::Serialize;
use ts_rs::{Config, TS};

/// Compile actual Serde output against generated types, not parallel handwritten fixtures.
pub(super) fn write(output: &Path, config: &Config) {
    let mut samples = String::new();
    let mut action = nanika_protocol::Action::primary("open", "Open");
    _sample(&action, config, &mut samples);
    action.icon = Some(nanika_protocol::ActionIcon::FolderOpen);
    action.confirmation_title = Some("Confirm".into());
    _sample(&action, config, &mut samples);
    let mut navigation = crate::NavigationSnapshot::default();
    _sample(&navigation, config, &mut samples);
    navigation.current = Some(None);
    _sample(&navigation, config, &mut samples);
    navigation.current = Some(Some(crate::ExtensionViewSnapshot {
        resource_scope: Some(nanika_host::ExtensionResourceScope {
            extension_id: "test.extension".into(),
            identity: "a".repeat(64)
        }),
        extension_name: "Test extension".into(),
        extension_icon_url: "icon.png".into(),
        route_id: 1,
        extension_id: "test.extension".into(),
        instance_id: 2,
        generation: 3,
        view_id: "detail".into(),
        revision: 4,
        view: std::sync::Arc::new(nanika_protocol::View::Detail {
            detail: nanika_protocol::DetailView {
                title: None,
                content: nanika_protocol::DetailContent::Text {
                    value: "Text".into(),
                    document: nanika_protocol::TextDocumentIndex::new(
                        "text".into(),
                        "fixture".into(),
                        &"Text"
                    )
                    .unwrap()
                    .document,
                    syntax_highlighting: false
                },
                metadata: Vec::new(),
                actions: vec![action]
            }
        })
    }));
    _sample(&navigation, config, &mut samples);
    let mut search = crate::RootSearchSnapshot {
        localization: Some(Default::default()),
        text_invalidations: Vec::new(),
        navigation,
        session_id: 9_007_199_254_740_991,
        request_id: 1,
        revision: 1,
        query: String::new(),
        result_revision: 1,
        result_offset: 0,
        total_results: 0,
        results: None,
        phase: crate::SearchPhase::Ready,
        error: None,
        warnings: Vec::new(),
        pending_extensions: Vec::new()
    };
    _sample(&search, config, &mut samples);
    search.results = Some(Vec::new());
    _sample(&search, config, &mut samples);
    for error in [
        nanika_host::ConfigurationSaveError::rejected(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::OperationPending
        )),
        nanika_host::ConfigurationSaveError::unconfirmed(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::ConfigurationUnconfirmed
        )),
        nanika_host::ConfigurationSaveError::SaveFailed {
            detail: "disk full".into()
        },
        nanika_host::ConfigurationSaveError::AppliedButNotSaved {
            detail: "disk full".into()
        },
        nanika_host::ConfigurationSaveError::Unavailable,
        nanika_host::ConfigurationSaveError::UnconfirmedNotSaved {
            cause: Box::new(nanika_host::OperationError::new(
                nanika_host::OperationErrorCode::ConfigurationUnconfirmed
            ))
        },
        nanika_host::ConfigurationSaveError::SavedButNotApplied {
            cause: Box::new(nanika_host::OperationError::with_detail(
                nanika_host::OperationErrorCode::ExtensionIo,
                "permission denied"
            ))
        }
    ] {
        _sample(&error, config, &mut samples);
        let command = crate::SettingsOperationError::Configuration(error.clone());
        assert_eq!(
            serde_json::to_value(command).unwrap(),
            serde_json::to_value(error).unwrap()
        );
    }
    for delivery_id in [None, Some(1)] {
        _sample(
            &crate::SettingsEvent::Application {
                delivery_id,
                update: crate::SettingsApplicationUpdate {
                    request_id: 1,
                    extension_id: "test.extension".into(),
                    resource_scope: None,
                    key: "enabled".into(),
                    result: crate::SettingsSaveResult::Completed {
                        outcome: nanika_host::ConfigurationSaveOutcome {
                            revision: 1,
                            values: [(
                                "nested".into(),
                                serde_json::json!([null, true, 1, {"x": "y"}])
                            )]
                            .into(),
                            saved: Default::default(),
                            effective: None,
                            error: None
                        }
                    }
                }
            },
            config,
            &mut samples
        );
    }
    for section in crate::general_settings::sections() {
        for field in &section.fields {
            assert_eq!(serde_json::to_value(field.key).unwrap(), field.key.as_str());
        }
        _sample(&section, config, &mut samples);
    }
    for (status, expected) in [
        (nanika_platform::StartupStatus::Disabled, "disabled"),
        (nanika_platform::StartupStatus::Enabled, "enabled"),
        (
            nanika_platform::StartupStatus::RequiresApproval,
            "requiresApproval"
        ),
        (nanika_platform::StartupStatus::NeedsRepair, "needsRepair"),
        (nanika_platform::StartupStatus::NotFound, "notFound")
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), expected);
        _sample(&status, config, &mut samples);
    }
    // These negative cases ensure the checker is actually using precise wire types.
    samples.push_str(concat!(
        "type HostMessage = import('./generated/LocalizedMessage').LocalizedMessage;\n",
        "({message: 'settings-title'}) satisfies HostMessage;\n",
        "({message: 'settings-search-count', args: {count: 2}}) satisfies HostMessage;\n",
        "({message: 'settings-shortcut-modifiers'}) satisfies HostMessage;\n",
        "// @ts-expect-error IDs come from admitted FTL, not a handwritten registry.\n",
        "({message: 'unknown-message'}) satisfies HostMessage;\n",
        "// @ts-expect-error A required argument object cannot be omitted.\n",
        "({message: 'settings-search-count'}) satisfies HostMessage;\n",
        "// @ts-expect-error Required arguments cannot be omitted.\n",
        "({message: 'settings-search-count', args: {}}) satisfies HostMessage;\n",
        "// @ts-expect-error Arguments are strings or numbers, never arbitrary objects.\n",
        "({message: 'settings-search-count', args: {count: {value: 2}}}) satisfies HostMessage;\n",
        "// @ts-expect-error A parameterless message does not accept arguments.\n",
        "({message: 'settings-title', args: {count: 2}}) satisfies HostMessage;\n",
        "// @ts-expect-error Argument names come from this message's complete contract.\n",
        "({message: 'settings-search-count', args: {count: 2, extra: 1}}) satisfies HostMessage;\n",
        "// @ts-expect-error The caller cannot supply platform.\n",
        "({message: 'settings-shortcut-modifiers', args: {platform: 'windows'}}) satisfies HostMessage;\n",
        "declare const hostMessage: HostMessage;\n",
        "hostMessage satisfies import('./generated/LocalizedText').LocalizedText;\n",
        "// @ts-expect-error An omitted result list is not a nullable list.\n",
        "(null) satisfies import('./generated/RootSearchSnapshot').RootSearchSnapshot['results'];\n",
        "// @ts-expect-error Required nullable fields cannot be omitted.\n",
        "({id: 'open', title: 'Open', style: 'primary', enabled: true, allow_default_execution: true}) satisfies import('./generated/Action').Action;\n",
        "// @ts-expect-error JSON numbers are not JavaScript bigint.\n",
        "(1n) satisfies import('./generated/ApplicationSnapshot').ApplicationSnapshot['sessionId'];\n",
    ));
    std::fs::write(output, samples).expect("write serialization contract samples");
}

fn _sample<T: Serialize + TS + 'static>(value: &T, config: &Config, output: &mut String) {
    T::export_all(config).expect("export sample type");
    let name = T::name(config);
    let json = serde_json::to_string(value).expect("serialize sample");
    output.push_str(&format!(
        "({json}) satisfies import('./generated/{name}').{name};\n"
    ));
}
