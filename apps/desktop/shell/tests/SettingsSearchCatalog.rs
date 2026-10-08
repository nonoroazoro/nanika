use crate::{ExtensionLifecycle, SettingsSearchCatalog, SettingsSearchTarget};
use std::collections::BTreeMap;

#[test]
fn general_render_definitions_are_all_searchable_without_extensions() {
    for section in crate::general_settings::sections() {
        for field in section.fields {
            let results = SettingsSearchCatalog::new(&[]).search(field.title);
            assert_eq!(results[0].page_id, "general");
            assert_eq!(results[0].page_title, "General");
            assert_eq!(
                results[0].target,
                SettingsSearchTarget::Field {
                    key: field.key.as_str().into()
                }
            );
        }
    }
    assert_eq!(
        SettingsSearchCatalog::new(&[]).search("dark")[0].title,
        "Theme"
    );
    assert!(SettingsSearchCatalog::new(&[]).search("  / -- ").is_empty());
}

#[test]
fn added_fields_use_schema_metadata_and_stable_keys_not_saved_values() {
    let mut extension = _extension();
    let configuration = extension.configuration.as_mut().unwrap();
    let mut property = configuration.contribution.properties["folders"].clone();
    property.title = "New locations".into();
    configuration
        .contribution
        .properties
        .insert("newLocations".into(), property);
    let results =
        SettingsSearchCatalog::new(std::slice::from_ref(&extension)).search("new locations");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].target,
        SettingsSearchTarget::Field {
            key: "newLocations".into()
        }
    );
    assert!(
        SettingsSearchCatalog::new(&[extension])
            .search("private-secret")
            .is_empty()
    );
}

#[test]
fn matching_uses_all_terms_across_title_and_breadcrumb_with_stable_ranking() {
    let extension = _extension();
    let results =
        SettingsSearchCatalog::new(std::slice::from_ref(&extension)).search("applications folder");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].title, "Folders");
    assert_eq!(results[0].page_title, "Applications");
    assert_eq!(results[1].title, "Other folders");
    let results =
        SettingsSearchCatalog::new(std::slice::from_ref(&extension)).search("APPLICATIONS");
    assert_eq!(results[0].target, SettingsSearchTarget::Page);
    assert!(
        SettingsSearchCatalog::new(std::slice::from_ref(&extension))
            .search("folders unknown")
            .is_empty()
    );
    assert!(
        SettingsSearchCatalog::new(&[extension])
            .search("目录")
            .is_empty()
    );
}

#[test]
fn platform_filtering_and_configuration_failures_match_visible_rows() {
    let mut extension = _extension();
    let configuration = extension.configuration.as_mut().unwrap();
    configuration
        .contribution
        .properties
        .get_mut("folders")
        .unwrap()
        .platforms = vec![nanika_platform::TargetPlatform::Macos];
    configuration.contribution = configuration
        .contribution
        .for_platform(nanika_platform::TargetPlatform::Windows);
    let results = SettingsSearchCatalog::new(std::slice::from_ref(&extension)).search("folders");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Other folders");
    extension.info.configuration_error = Some("Invalid configuration".into());
    assert!(
        SettingsSearchCatalog::new(std::slice::from_ref(&extension))
            .search("folders")
            .is_empty()
    );
    assert_eq!(
        SettingsSearchCatalog::new(std::slice::from_ref(&extension)).search("applications")[0]
            .target,
        SettingsSearchTarget::Page
    );
    assert_eq!(
        SettingsSearchCatalog::new(&[extension]).search("enable applications")[0].target,
        SettingsSearchTarget::Enabled
    );
}

#[test]
fn renamed_labels_keep_destinations_and_no_search_metadata_leaks_over_ipc() {
    let mut extension = _extension();
    let before = SettingsSearchCatalog::new(std::slice::from_ref(&extension))
        .search("Folders")
        .remove(0);
    extension
        .configuration
        .as_mut()
        .unwrap()
        .contribution
        .properties
        .get_mut("folders")
        .unwrap()
        .title = "Places".into();
    let after = SettingsSearchCatalog::new(&[extension])
        .search("Places")
        .remove(0);
    assert_eq!(before.target, after.target);
    let json = serde_json::to_value(after).unwrap();
    assert_eq!(
        json["target"],
        serde_json::json!({"kind": "field", "key": "folders"})
    );
    assert_eq!(json["pageTitle"], "Applications");
    assert!(json.get("searchValues").is_none());
}

#[test]
fn settings_and_app_list_share_matching_except_romanized_aliases() {
    let mut extension = _extension();
    extension.info.name = "Calculator".into();
    extension.configuration = None;
    for query in ["calculator", "CAL", "clc", "calc-ulator", "unrelated"] {
        let candidate = nanika_search::Candidate::new(
            nanika_search::CandidateKind::Action,
            "test.application",
            "calculator",
            "Calculator",
            "launch",
            vec![nanika_protocol::Action::primary("launch", "Open")],
            vec![],
        );
        let app_match = !nanika_search::SearchEngine::new()
            .query(query, &[candidate], &nanika_search::UsageMap::new(), 0)
            .results
            .is_empty();
        let settings_match = SettingsSearchCatalog::new(std::slice::from_ref(&extension))
            .search(query)
            .iter()
            .any(|entry| {
                entry.page_id == extension.info.id && entry.target == SettingsSearchTarget::Page
            });
        assert_eq!(settings_match, app_match, "{query}");
    }
    extension.info.name = "音乐".into();
    assert!(
        SettingsSearchCatalog::new(std::slice::from_ref(&extension))
            .search("音乐")
            .iter()
            .any(|entry| entry.page_id == extension.info.id)
    );
    assert!(
        SettingsSearchCatalog::new(&[extension])
            .search("yinyue")
            .is_empty()
    );
}

#[test]
fn descriptions_do_not_admit_unrelated_fuzzy_matches() {
    let mut extension = _extension();
    extension
        .configuration
        .as_mut()
        .unwrap()
        .contribution
        .properties
        .get_mut("folders")
        .unwrap()
        .description = Some("Search these folders in addition to your system applications.".into());
    let results = SettingsSearchCatalog::new(&[extension]).search("them");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Theme");
    assert_eq!(results[0].page_id, "general");
    assert_eq!(
        SettingsSearchCatalog::new(&[]).search("dark")[0].title,
        "Theme"
    );
}

#[test]
fn about_and_version_queries_reveal_the_host_page_without_extensions() {
    for query in ["about", "version", "nanika"] {
        let results = SettingsSearchCatalog::new(&[]).search(query);
        assert_eq!(results[0].page_id, "about");
        assert_eq!(results[0].title, "About");
        assert_eq!(results[0].target, SettingsSearchTarget::Page);
    }
}

fn _extension() -> ExtensionLifecycle {
    let contribution = serde_json::from_value(serde_json::json!({
        "title": "Applications",
        "properties": {
            "folders": {"title": "Folders", "description": "Extra search locations 目录", "type": "string", "maxLength": 512, "default": "", "persistence": "beforeApply", "order": 1},
            "other": {"title": "Other folders", "type": "string", "maxLength": 512, "default": "", "persistence": "beforeApply", "order": 2}
        }
    })).unwrap();
    ExtensionLifecycle {
        info: nanika_host::RuntimeExtensionInfo {
            id: "test.applications".into(),
            name: "Applications".into(),
            enabled: false,
            pending: false,
            state: nanika_host::RuntimeExtensionState::Disabled,
            instance_id: None,
            lifecycle_error: None,
            configuration_error: None,
            icon: "icon.png".into(),
            icon_hash: None,
        },
        icon_url: "".into(),
        configuration: Some(nanika_host::RuntimeExtensionConfiguration {
            revision: 0,
            extension_id: "test.applications".into(),
            contribution,
            values: BTreeMap::from([("folders".into(), serde_json::json!("private-secret"))]),
            saved: BTreeMap::new(),
            effective: None,
        }),
    }
}
