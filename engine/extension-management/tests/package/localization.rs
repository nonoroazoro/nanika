use super::*;
use nanika_extension_package::{InstalledExtension, parse_extension_manifest};
use nanika_localization::SupportedLocale;
use nanika_protocol::message;
use std::path::Path;

#[test]
fn invalid_localization_updates_preserve_the_active_package_and_registry() {
    let root = temporary_root("localization-rollback");
    cleanup(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = NanikaPaths::from_roots(root.join("data"), root.join("cache"), root.join("config"));
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).unwrap();
    let package = root.join("example.nanika");
    _package(
        &package,
        "1.0.0",
        &_declaration(),
        &[("locales/en.ftl", "title = Before")]
    );
    let installed = install_package(&package, &paths, &store, &[]).unwrap();
    let retained = installed.localization.unwrap();
    set_extension_enabled("com.example.extension", false, &paths, &store).unwrap();
    let database = HostDatabase::open(paths.host_database()).unwrap();
    let before = database.extension("com.example.extension").unwrap();
    let registry = std::fs::read(paths.config_root().join("extensions.jsonc")).unwrap();
    let mut invalid_sources = [
        "title = { missing }",
        "title = { title }",
        "title = First\ntitle = Duplicate",
        "title = { NUMBER($count) }",
        "title = {"
    ]
    .map(str::to_owned)
    .to_vec();
    invalid_sources.push("x".repeat(nanika_localization::MAX_RESOURCE_BYTES + 1));
    for source in &invalid_sources {
        _package(
            &package,
            "1.1.0",
            &_declaration(),
            &[("locales/en.ftl", source)]
        );
        let error = update_package(&package, &paths, &store, &[])
            .unwrap_err()
            .to_string();
        assert!(error.contains("com.example.extension"), "{error}");
        assert!(error.contains("locales/en.ftl"), "{error}");
        assert_eq!(database.extension("com.example.extension").unwrap(), before);
        assert_eq!(
            std::fs::read(paths.config_root().join("extensions.jsonc")).unwrap(),
            registry
        );
        assert!(
            !paths
                .app_data_root()
                .join("extensions/com.example.extension/1.1.0")
                .exists()
        );
        assert_eq!(
            retained
                .format(
                    SupportedLocale::English,
                    &message("title", []),
                    "macos",
                    128,
                    false
                )
                .unwrap(),
            "Before"
        );
    }
    assert!(
        std::fs::read_dir(paths.app_data_root().join("extensions"))
            .unwrap()
            .all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with('.')
            })
    );
    drop(database);
    cleanup(&root);
}

#[test]
fn admitted_resources_survive_update_and_removed_source_files() {
    let root = temporary_root("localization-lifetime");
    cleanup(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = NanikaPaths::from_roots(root.join("data"), root.join("cache"), root.join("config"));
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).unwrap();
    let package = root.join("example.nanika");
    _package(
        &package,
        "1.0.0",
        &_declaration(),
        &[("locales/en.ftl", "title = Before")]
    );
    let old = install_package(&package, &paths, &store, &[]).unwrap();
    let retained = old.localization.unwrap();
    _package(
        &package,
        "1.1.0",
        &_declaration(),
        &[("locales/en.ftl", "title = After")]
    );
    let new = update_package(&package, &paths, &store, &[]).unwrap();
    let current = new.localization.unwrap();
    assert_ne!(retained.prepared().identity, current.prepared().identity);
    let database = HostDatabase::open(paths.host_database()).unwrap();
    let records = database.load_extensions().unwrap();
    let (resolved, errors) = resolve_installed_extensions(&paths, &records, &[]);
    assert!(errors.is_empty());
    assert_eq!(
        resolved[0]
            .localization
            .as_ref()
            .unwrap()
            .prepared()
            .identity,
        current.prepared().identity
    );
    std::fs::remove_file(old.resource_root.join("locales/en.ftl")).unwrap();
    std::fs::remove_file(new.resource_root.join("locales/en.ftl")).unwrap();
    for (resources, title) in [(retained, "Before"), (current, "After")] {
        // A one-language external package falls back without consulting mutable package paths.
        assert_eq!(
            resources
                .format(
                    SupportedLocale::SimplifiedChinese,
                    &message("title", []),
                    "windows",
                    128,
                    false
                )
                .unwrap(),
            title
        );
    }
    let (resolved, errors) = resolve_installed_extensions(&paths, &records, &[]);
    assert!(resolved.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("locales/en.ftl"));
    assert_eq!(database.load_extensions().unwrap(), records);
    #[cfg(unix)]
    {
        let outside = root.join("outside.ftl");
        std::fs::write(&outside, "title = Outside").unwrap();
        std::os::unix::fs::symlink(outside, new.resource_root.join("locales/en.ftl")).unwrap();
        let (resolved, errors) = resolve_installed_extensions(&paths, &records, &[]);
        assert!(resolved.is_empty());
        assert!(errors[0].message.contains("escaped its package root"));
    }
    drop(database);
    cleanup(&root);
}

#[test]
fn declarations_and_missing_resources_fail_before_installation() {
    let root = temporary_root("localization-declarations");
    cleanup(&root);
    std::fs::create_dir_all(&root).unwrap();
    let paths = NanikaPaths::from_roots(root.join("data"), root.join("cache"), root.join("config"));
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).unwrap();
    let package = root.join("example.nanika");
    for (declaration, expected) in [
        (_declaration(), "locales/en.ftl"),
        (
            serde_json::json!({"defaultLocale":"zh-Hans", "resources":{"en":["locales/en.ftl"]}}),
            "Default locale must be declared"
        ),
        (
            serde_json::json!({"defaultLocale":"en", "resources":{"en":["../outside.ftl"]}}),
            "invalid or duplicate relative FTL path"
        ),
        (
            serde_json::json!({"defaultLocale":"en", "resources":{"en":["https://example.org/en.ftl"]}}),
            "invalid or duplicate relative FTL path"
        ),
        (
            serde_json::json!({"defaultLocale":"en", "resources":{"en":["locales/en.ftl", "locales/en.ftl"]}}),
            "invalid or duplicate relative FTL path"
        ),
        (
            serde_json::json!({"defaultLocale":"fr", "resources":{"fr":["locales/fr.ftl"]}}),
            "unknown variant"
        )
    ] {
        _package(&package, "1.0.0", &declaration, &[]);
        let error = install_package(&package, &paths, &store, &[])
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
        assert!(
            HostDatabase::open(paths.host_database())
                .unwrap()
                .load_extensions()
                .unwrap()
                .is_empty()
        );
        assert!(
            !paths
                .app_data_root()
                .join("extensions/com.example.extension")
                .exists()
        );
    }
    cleanup(&root);
}

#[test]
fn built_in_admission_uses_the_same_resources_with_complete_translation_checks() {
    let root = temporary_root("localization-built-in");
    cleanup(&root);
    std::fs::create_dir_all(root.join("locales")).unwrap();
    std::fs::write(root.join("locales/en.ftl"), "title = Hello\nextra = Extra").unwrap();
    let manifest =
        parse_extension_manifest(&_manifest("1.0.0", &_declaration()).to_string()).unwrap();
    assert!(
        InstalledExtension::from_manifest(
            manifest.clone(),
            root.join("bin/example"),
            root.clone(),
            false
        )
        .is_ok()
    );
    assert!(
        InstalledExtension::from_manifest(manifest, root.join("bin/example"), root.clone(), true)
            .is_err()
    );
    let declaration = serde_json::json!({"defaultLocale":"en", "resources":{"en":["locales/en.ftl"], "zh-Hans":["locales/zh-Hans.ftl"]}});
    let manifest = parse_extension_manifest(&_manifest("1.0.0", &declaration).to_string()).unwrap();
    std::fs::write(root.join("locales/zh-Hans.ftl"), "title = 你好").unwrap();
    assert!(
        InstalledExtension::from_manifest(
            manifest.clone(),
            root.join("bin/example"),
            root.clone(),
            true
        )
        .is_err()
    );
    std::fs::write(
        root.join("locales/zh-Hans.ftl"),
        "title = 你好\nextra = 更多"
    )
    .unwrap();
    let built_in = InstalledExtension::from_manifest(
        manifest.clone(),
        root.join("bin/example"),
        root.clone(),
        true
    )
    .unwrap();
    let external =
        InstalledExtension::from_manifest(manifest, root.join("bin/example"), root.clone(), false)
            .unwrap();
    assert_eq!(
        built_in.localization.unwrap().prepared().identity,
        external.localization.unwrap().prepared().identity
    );
    cleanup(&root);
}

#[test]
fn localized_names_validate_all_platforms_and_locales_before_admission() {
    let root = temporary_root("localized-names");
    cleanup(&root);
    std::fs::create_dir_all(root.join("locales")).unwrap();
    let declaration = serde_json::json!({"defaultLocale":"en", "resources":{"en":["locales/en.ftl"], "zh-Hans":["locales/zh-Hans.ftl"]}});
    let mut manifest = _manifest("1.0.0", &declaration);
    manifest["name"] = serde_json::json!({"message":"title"});
    std::fs::write(
        root.join("locales/en.ftl"),
        "title = Applications\nother = Extra"
    )
    .unwrap();
    std::fs::write(root.join("locales/zh-Hans.ftl"), "title = 应用程序").unwrap();
    let parsed = parse_extension_manifest(&manifest.to_string()).unwrap();
    let admitted =
        InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
            .unwrap();
    assert_eq!(admitted.name, message("title", []));
    assert_eq!(
        admitted.name_search_values,
        vec!["Applications", "应用程序"]
    );
    let mut unknown = parsed.clone();
    unknown.name = message("missing-title", []);
    assert!(
        InstalledExtension::from_manifest(unknown, "unused".into(), root.clone(), false).is_err()
    );
    std::fs::write(
        root.join("locales/zh-Hans.ftl"),
        "other = Missing translation"
    )
    .unwrap();
    let fallback =
        InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
            .unwrap();
    assert_eq!(fallback.name_search_values, vec!["Applications"]);
    for source in [
        "title = { $missing }".to_owned(),
        format!("title = {}", "界".repeat(43)),
        "title = First\n    Second".to_owned(),
        format!(
            "title = {{ $platform ->\n    [macos] Valid\n   *[windows] {}\n}}",
            "x".repeat(129)
        )
    ] {
        std::fs::write(root.join("locales/zh-Hans.ftl"), source).unwrap();
        let error =
            InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
                .unwrap_err()
                .to_string();
        assert!(error.contains("com.example.extension"), "{error}");
        assert!(
            error.contains("name") || error.contains("localization"),
            "{error}"
        );
    }
    manifest.as_object_mut().unwrap().remove("localization");
    assert!(parse_extension_manifest(&manifest.to_string()).is_err());
    cleanup(&root);
}

#[test]
fn configuration_messages_are_admitted_with_bilingual_search_data_and_original_value_contract() {
    let root = temporary_root("localized-configuration");
    cleanup(&root);
    std::fs::create_dir_all(root.join("locales")).unwrap();
    let declaration = serde_json::json!({"defaultLocale":"en", "resources":{"en":["locales/en.ftl"], "zh-Hans":["locales/zh-Hans.ftl"]}});
    let mut manifest = _manifest("1.0.0", &declaration);
    manifest["contributes"]["configuration"] = serde_json::json!({
        "title":{"message":"section"},
        "properties":{"enabled":{"title":{"message":"toggle"}, "description":{"message":"description"},
            "group":{"id":"appearance", "title":{"message":"section"}},
            "type":"boolean", "default":true, "persistence":"beforeApply"}},
        "quickMenu":[{"id":"appearance", "title":{"message":"section"}, "items":[{"kind":"toggle", "key":"enabled"}]}]
    });
    std::fs::write(
        root.join("locales/en.ftl"),
        "section = Appearance\ntoggle = Colors\ndescription = Highlight recognized code."
    )
    .unwrap();
    let chinese = "section = 外观\ntoggle = 色彩\ndescription = 高亮显示识别出的代码。";
    std::fs::write(root.join("locales/zh-Hans.ftl"), chinese).unwrap();
    let parsed = parse_extension_manifest(&manifest.to_string()).unwrap();
    let admitted =
        InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
            .unwrap();
    assert_eq!(
        admitted.configuration_search_values["field:enabled"],
        vec!["Colors", "Appearance", "色彩", "外观"]
    );
    let contribution = admitted.contributes.configuration.unwrap();
    assert_eq!(contribution.defaults()["enabled"], true);
    assert!(matches!(
        contribution.properties["enabled"].title,
        nanika_protocol::LocalizedText::Message(_)
    ));
    for source in [
        "section = 外观\ntoggle = { $missing }\ndescription = Description".to_owned(),
        format!(
            "section = 外观\ntoggle = {}\ndescription = Description",
            "界".repeat(43)
        ),
        format!(
            "section = 外观\ntoggle = Color\ndescription = {}",
            "界".repeat(171)
        ),
        format!(
            "section = 外观\ntoggle = {{ $platform ->\n [macos] Color\n *[windows] {}\n}}\ndescription = Description",
            "x".repeat(129)
        )
    ] {
        std::fs::write(root.join("locales/zh-Hans.ftl"), source).unwrap();
        assert!(
            InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
                .is_err()
        );
    }
    std::fs::write(root.join("locales/zh-Hans.ftl"), chinese).unwrap();
    let mut missing = parsed.clone();
    missing
        .contributes
        .configuration
        .as_mut()
        .unwrap()
        .properties
        .get_mut("enabled")
        .unwrap()
        .title = message("unknown", []);
    assert!(
        InstalledExtension::from_manifest(missing, "unused".into(), root.clone(), false).is_err()
    );
    let mut undeclared = parsed;
    undeclared.localization = None;
    assert!(
        InstalledExtension::from_manifest(undeclared, "unused".into(), root.clone(), false)
            .is_err()
    );
    cleanup(&root);
}

#[test]
fn action_messages_share_admission_and_keep_search_targets_and_group_identity_distinct() {
    let root = temporary_root("localized-actions");
    cleanup(&root);
    std::fs::create_dir_all(root.join("locales")).unwrap();
    for (locale, source) in [
        ("en", "reset = Reset\ngroup = Options"),
        ("zh-Hans", "reset = 重置\ngroup = 选项")
    ] {
        std::fs::write(root.join(format!("locales/{locale}.ftl")), source).unwrap();
    }
    let mut manifest = _manifest(
        "1.0.0",
        &serde_json::json!({"defaultLocale":"en", "resources":{"en":["locales/en.ftl"], "zh-Hans":["locales/zh-Hans.ftl"]}})
    );
    manifest["contributes"]["configuration"] = serde_json::json!({
        "title":"Options", "properties":{"enabled":{"type":"boolean", "title":"Enabled", "default":true, "persistence":"beforeApply"}},
        "actions":[{"description":"Reset configuration.", "action":{"id":"enabled", "title":{"message":"reset"}, "style":"secondary", "enabled":true, "allow_default_execution":false,
            "group":{"id":"options", "title":{"message":"group"}}}}]
    });
    let mut parsed = parse_extension_manifest(&manifest.to_string()).unwrap();
    let admitted =
        InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
            .unwrap();
    assert_eq!(
        admitted.configuration_search_values["field:enabled"],
        ["Enabled", "Options"]
    );
    assert_eq!(
        admitted.configuration_search_values["action:enabled"],
        ["Reset", "Options", "重置", "选项"]
    );
    let mut action = parsed.contributes.configuration.as_ref().unwrap().actions[0]
        .action
        .clone();
    action.id = nanika_protocol::COMMAND_EXECUTE_ACTION_ID.into();
    parsed
        .contributes
        .commands
        .push(crate::CommandContribution {
            command: "reset".into(),
            title: "Reset".into(),
            description: "Reset fixture".into(),
            action,
            category: None,
            keywords: Vec::new(),
            icon: None
        });
    InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
        .unwrap();
    parsed.contributes.commands[0]
        .action
        .group
        .as_mut()
        .unwrap()
        .title = "Conflicting".into();
    assert!(
        InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
            .unwrap_err()
            .to_string()
            .contains("conflicting")
    );
    parsed.contributes.commands[0].action.group = None;
    parsed.contributes.commands[0].action.title = message("missing", []);
    assert!(
        InstalledExtension::from_manifest(parsed, "unused".into(), root.clone(), false)
            .unwrap_err()
            .to_string()
            .contains("unknown message")
    );
    cleanup(&root);
}

#[test]
fn contribution_messages_validate_every_destination_in_both_platform_branches() {
    let root = temporary_root("localized-contributions");
    cleanup(&root);
    std::fs::create_dir_all(root.join("locales")).unwrap();
    let valid = "title = Title\ndescription = Description\ncategory = Category\nkeyword = Keyword";
    std::fs::write(root.join("locales/en.ftl"), valid).unwrap();
    let mut manifest = _manifest(
        "1.0.0",
        &serde_json::json!({
            "defaultLocale":"en", "resources":{"en":["locales/en.ftl"], "zh-Hans":["locales/zh-Hans.ftl"]}
        })
    );
    for kind in ["commands", "views"] {
        let mut contribution = serde_json::json!({
            "title":{"message":"title"}, "description":{"message":"description"},
            "category":{"message":"category"}, "keywords":[{"message":"keyword"}]
        });
        if kind == "commands" {
            contribution["command"] = serde_json::json!("example.command");
            contribution["action"] = serde_json::to_value(nanika_protocol::Action::primary(
                nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                "Run"
            ))
            .unwrap();
        } else {
            contribution["id"] = serde_json::json!("example.view");
        }
        manifest["contributes"] = serde_json::json!({kind: [contribution]});
        let parsed = parse_extension_manifest(&manifest.to_string()).unwrap();
        std::fs::write(root.join("locales/zh-Hans.ftl"), valid).unwrap();
        InstalledExtension::from_manifest(parsed.clone(), "unused".into(), root.clone(), false)
            .unwrap();
        for (field, limit) in [
            ("title", 128),
            ("description", 512),
            ("category", 128),
            ("keyword", 64)
        ] {
            let invalid = valid.lines().map(|line| {
                if line.starts_with(&format!("{field} =")) {
                    format!("{field} = {{ $platform ->\n    [macos] Valid\n   *[windows] {}\n    }}", "界".repeat(limit + 1))
                } else { line.to_owned() }
            }).collect::<Vec<_>>().join("\n");
            std::fs::write(root.join("locales/zh-Hans.ftl"), invalid).unwrap();
            let error = InstalledExtension::from_manifest(
                parsed.clone(),
                "unused".into(),
                root.clone(),
                false
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains(field), "{kind}: {field}: {error}");
        }
    }
    cleanup(&root);
}

fn _declaration() -> serde_json::Value {
    serde_json::json!({"defaultLocale":"en", "resources":{"en":["locales/en.ftl"]}})
}

fn _manifest(version: &str, declaration: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "format": "nanika-extension",
        "manifestVersion": 1,
        "id": "com.example.extension",
        "name": "Example",
        "icon": "assets/icon.png",
        "version": version,
        "hostApi": "^0.1",
        "runtime": {"protocol":"nanika", "protocolVersion":1},
        "targets": {(nanika_platform::target_triple()): {"entrypoint":format!("bin/{}/example{}", nanika_platform::target_triple(), std::env::consts::EXE_SUFFIX)}},
        "localization": declaration
    })
}

fn _package(path: &Path, version: &str, declaration: &serde_json::Value, files: &[(&str, &str)]) {
    let mut archive = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let manifest = _manifest(version, declaration);
    let entrypoint = manifest["targets"][nanika_platform::target_triple()]["entrypoint"]
        .as_str()
        .unwrap();
    let source = manifest.to_string();
    for (name, source) in [("manifest.jsonc", source.as_str()), (entrypoint, "fixture")]
        .into_iter()
        .chain(files.iter().copied())
    {
        archive
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(source.as_bytes()).unwrap();
    }
    archive.finish().unwrap();
}
