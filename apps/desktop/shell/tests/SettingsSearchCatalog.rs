use crate::{ExtensionLifecycle, SettingsSearchCatalog, SettingsSearchTarget};
use std::collections::BTreeMap;

#[test]
fn general_render_definitions_are_all_searchable_without_extensions() {
    for section in crate::general_settings::sections() {
        for field in section.fields {
            let results = SettingsSearchCatalog::new(&[]).search(&_text(&field.title));
            assert_eq!(results[0].page_id, "general");
            assert_eq!(_text(&results[0].page_title), "General");
            assert_eq!(
                results[0].target,
                SettingsSearchTarget::Field {
                    key: field.key.as_str().into()
                }
            );
        }
    }
    assert_eq!(
        _text(&SettingsSearchCatalog::new(&[]).search("dark")[0].title),
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
fn matching_uses_all_terms_across_title_and_breadcrumb_with_page_order() {
    let extension = _extension();
    let results =
        SettingsSearchCatalog::new(std::slice::from_ref(&extension)).search("applications folder");
    assert_eq!(results.len(), 2);
    assert_eq!(_text(&results[0].title), "Folders");
    assert_eq!(_text(&results[0].page_title), "Applications");
    assert_eq!(_text(&results[1].title), "Other folders");
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
    assert_eq!(_text(&results[0].title), "Other folders");
    extension.info.configuration_error = Some(nanika_host::OperationError::with_detail(
        nanika_host::OperationErrorCode::ExtensionConfigurationUnavailable,
        "Invalid configuration"
    ));
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
    extension.info.name_search_values = vec!["Calculator".into()];
    extension.configuration = None;
    for query in ["calculator", "CAL", "clc", "calc-ulator", "unrelated"] {
        let candidate = nanika_search::Candidate::new(
            nanika_search::CandidateKind::Action,
            "test.application",
            "calculator",
            "Calculator",
            "launch",
            vec![nanika_protocol::Action::primary("launch", "Open")],
            vec![]
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
    extension.info.name_search_values = vec!["音乐".into()];
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
    assert_eq!(_text(&results[0].title), "Theme");
    assert_eq!(results[0].page_id, "general");
    assert_eq!(
        _text(&SettingsSearchCatalog::new(&[]).search("dark")[0].title),
        "Theme"
    );
}

#[test]
fn about_and_version_queries_reveal_the_host_page_without_extensions() {
    for query in ["about", "关于", "version", "nanika"] {
        let results = SettingsSearchCatalog::new(&[]).search(query);
        // Other fuzzy matches may precede About in navigation order.
        let about = results
            .iter()
            .find(|entry| entry.page_id == "about")
            .unwrap();
        assert_eq!(_text(&about.title), "About");
        assert_eq!(about.title, nanika_protocol::message("settings-about", []));
        assert_eq!(about.page_title, about.title);
        assert!(about.title_resource_scope.is_none());
        assert!(about.page_resource_scope.is_none());
        assert_eq!(
            crate::host_resources::resources()
                .unwrap()
                .format(
                    nanika_localization::SupportedLocale::SimplifiedChinese,
                    &about.title,
                    "windows",
                    512,
                    false
                )
                .unwrap(),
            "关于"
        );
        assert_eq!(about.target, SettingsSearchTarget::Page);
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
            resource_scope: None,
            id: "test.applications".into(),
            name: "Applications".into(),
            name_search_values: vec!["Applications".into()],
            configuration_search_values: Default::default(),
            enabled: false,
            pending: false,
            state: nanika_host::RuntimeExtensionState::Disabled,
            instance_id: None,
            lifecycle_error: None,
            configuration_error: None,
            icon: "icon.png".into(),
            icon_hash: None
        },
        icon_url: "".into(),
        configuration: Some(nanika_host::RuntimeExtensionConfiguration {
            revision: 0,
            extension_id: "test.applications".into(),
            contribution,
            values: BTreeMap::from([("folders".into(), serde_json::json!("private-secret"))]),
            saved: BTreeMap::new(),
            effective: None
        })
    }
}

#[test]
fn declared_settings_actions_are_searchable_by_their_group_and_title() {
    let mut extension = _extension();
    extension
        .configuration
        .as_mut()
        .unwrap()
        .contribution
        .actions
        .push(
            serde_json::from_value(serde_json::json!({
                "description": "Reset test data.",
                "action": { "id": "reset", "title": "Reset data", "style": "destructive",
                    "enabled": true, "allow_default_execution": false,
                    "confirmation_title": "Reset now?", "group": {"id":"danger-zone","title":"Danger zone"} }
            }))
            .unwrap()
        );
    let results = SettingsSearchCatalog::new(&[extension]).search("danger reset");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].target,
        SettingsSearchTarget::Action {
            key: "reset".into()
        }
    );
}

#[test]
fn matches_keep_visual_field_order_even_when_later_titles_match_better() {
    let mut extension = _extension();
    let properties = &mut extension
        .configuration
        .as_mut()
        .unwrap()
        .contribution
        .properties;
    properties.get_mut("folders").unwrap().title = "Extra folders".into();
    properties.get_mut("other").unwrap().title = "Folders".into();
    let results = SettingsSearchCatalog::new(&[extension]).search("folders");
    assert_eq!(
        results
            .iter()
            .map(|entry| _text(&entry.title))
            .collect::<Vec<_>>(),
        vec!["Extra folders", "Folders"]
    );
}

#[test]
fn extension_matches_follow_fields_then_actions_in_rendered_group_order() {
    let mut extension = _extension();
    let configuration = extension.configuration.as_mut().unwrap();
    // Deliberately disagree with both key order and match-score order.
    configuration
        .contribution
        .properties
        .get_mut("other")
        .unwrap()
        .order = 0;
    for (id, group) in [
        ("first", Some(("danger-zone", "Danger zone"))),
        ("second", None),
        ("third", Some(("danger-zone", "Danger zone"))),
        ("fourth", Some(("explicit-actions", "Actions"))),
        ("fifth", None)
    ] {
        let group = group.map(|(id, title)| serde_json::json!({"id": id, "title": title}));
        configuration.contribution.actions.push(
            serde_json::from_value(serde_json::json!({
                "description": "Test operation.",
                "action": {"id": id, "title": "Applications", "style": "secondary",
                    "enabled": true, "allow_default_execution": false, "group": group}
            }))
            .unwrap()
        );
    }
    let results = SettingsSearchCatalog::new(&[extension]).search("applications");
    assert_eq!(
        results
            .into_iter()
            .map(|entry| entry.target)
            .collect::<Vec<_>>(),
        vec![
            SettingsSearchTarget::Page,
            SettingsSearchTarget::Enabled,
            SettingsSearchTarget::Field {
                key: "other".into()
            },
            SettingsSearchTarget::Field {
                key: "folders".into()
            },
            SettingsSearchTarget::Action {
                key: "first".into()
            },
            SettingsSearchTarget::Action {
                key: "third".into()
            },
            SettingsSearchTarget::Action {
                key: "second".into()
            },
            SettingsSearchTarget::Action {
                key: "fifth".into()
            },
            SettingsSearchTarget::Action {
                key: "fourth".into()
            }
        ]
    );
}

#[test]
fn general_matches_follow_section_and_field_order() {
    let results = SettingsSearchCatalog::new(&[]).search("general");
    let mut expected = vec![SettingsSearchTarget::Page];
    for section in crate::general_settings::sections() {
        expected.push(SettingsSearchTarget::Section {
            key: section.key.into()
        });
        for field in section.fields {
            expected.push(SettingsSearchTarget::Field {
                key: field.key.as_str().into()
            });
        }
    }
    assert_eq!(
        results
            .into_iter()
            .map(|entry| entry.target)
            .collect::<Vec<_>>(),
        expected
    );
}

fn _text(text: &nanika_protocol::LocalizedText) -> String {
    crate::host_resources::resources()
        .unwrap()
        .format(
            nanika_localization::SupportedLocale::English,
            text,
            "macos",
            512,
            false
        )
        .unwrap()
}

#[test]
fn bilingual_names_keep_scoped_sources_and_identical_search_order() {
    let mut extension = _extension();
    extension.info.name = nanika_protocol::message("extension-name", []);
    extension.info.name_search_values = vec!["Applications".into(), "应用程序".into()];
    let scope = nanika_host::ExtensionResourceScope {
        extension_id: extension.info.id.clone(),
        identity: "a".repeat(64)
    };
    extension.info.resource_scope = Some(scope.clone());
    let catalog = SettingsSearchCatalog::new(&[extension]);
    let english = catalog.search("applications");
    let chinese = catalog.search("应用程序");
    assert_eq!(
        serde_json::to_value(&english).unwrap(),
        serde_json::to_value(&chinese).unwrap()
    );
    assert_eq!(
        english[0].title,
        nanika_protocol::message("extension-name", [])
    );
    assert_eq!(english[0].title_resource_scope, Some(scope.clone()));
    assert_eq!(english[0].page_resource_scope, Some(scope.clone()));
    assert_eq!(english[1].title_resource_scope, None);
    assert_eq!(english[1].page_resource_scope, Some(scope));
    assert!(
        serde_json::to_value(&english)
            .unwrap()
            .to_string()
            .find("nameSearchValues")
            .is_none()
    );
}

#[test]
fn localized_configuration_fields_use_prepared_aliases_and_explicit_package_scope() {
    let mut extension = _extension();
    extension.info.name_search_values.push("应用程序".into());
    extension.info.resource_scope = Some(nanika_host::ExtensionResourceScope {
        extension_id: extension.info.id.clone(),
        identity: "b".repeat(64)
    });
    let property = extension
        .configuration
        .as_mut()
        .unwrap()
        .contribution
        .properties
        .get_mut("folders")
        .unwrap();
    property.title = nanika_protocol::message("folders-title", []);
    property.group = Some(nanika_protocol::PresentationGroup {
        id: "locations".into(),
        title: nanika_protocol::message("locations-title", [])
    });
    extension.info.configuration_search_values.insert(
        "field:folders".into(),
        vec![
            "Folders".into(),
            "Search locations".into(),
            "文件夹".into(),
            "搜索位置".into(),
        ]
    );
    assert!(
        serde_json::to_value(&extension.info)
            .unwrap()
            .get("configurationSearchValues")
            .is_none()
    );
    let catalog = SettingsSearchCatalog::new(&[extension]);
    let chinese = catalog.search("应用程序 文件夹");
    let english = catalog.search("applications locations");
    assert_eq!(
        serde_json::to_value(&english).unwrap(),
        serde_json::to_value(&chinese).unwrap()
    );
    assert_eq!(chinese.len(), 1);
    assert_eq!(
        chinese[0].title,
        nanika_protocol::message("folders-title", [])
    );
    assert_eq!(
        chinese[0]
            .title_resource_scope
            .as_ref()
            .unwrap()
            .extension_id,
        "test.applications"
    );
}
