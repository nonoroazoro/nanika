use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn supplied_names_survive_scanning_cache_reopen_and_locale_independent_search() {
    let root = _root("search");
    let bundle = root.join("NeteaseMusic.app");
    let contents = bundle.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    std::fs::write(contents.join("MacOS/Music"), "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(
        contents.join("MacOS/Music"),
        std::fs::Permissions::from_mode(0o755)
    )
    .unwrap();
    plist::Value::Dictionary(
        [
            (
                "CFBundleIdentifier".to_owned(),
                plist::Value::String("test.native.names".to_owned())
            ),
            (
                "CFBundleExecutable".to_owned(),
                plist::Value::String("Music".to_owned())
            ),
            (
                "CFBundleName".to_owned(),
                plist::Value::String("NeteaseMusic".to_owned())
            )
        ]
        .into_iter()
        .collect()
    )
    .to_file_xml(contents.join("Info.plist"))
    .unwrap();
    _strings(&bundle, "en", "NetEaseMusic");
    _strings(&bundle, "zh-Hans", "网易云音乐");
    _strings(&bundle, "zh-Hant", "網易雲音樂");
    _strings(&bundle, "ja", "ミュージック");
    _strings(&bundle, "fr", "Musique");
    let mut state = crate::DiscoveryState::new();
    let entry = crate::platform::read_entry(&mut state, &bundle, 0)
        .unwrap()
        .unwrap();
    assert_eq!(entry.localized_names["en"], "NetEaseMusic");
    assert_eq!(entry.localized_names["zh-Hans"], "网易云音乐");
    let database_path = root.join("app.db");
    let mut database = crate::ApplicationDatabase::open(&database_path).unwrap();
    database
        .commit_source("root", std::slice::from_ref(&entry), &[])
        .unwrap();
    drop(database);
    let database = crate::ApplicationDatabase::open(&database_path).unwrap();
    let reloaded = database.load_entries().unwrap().remove(0);
    assert_eq!(entry, reloaded);
    let candidate = reloaded.candidate();
    let resources = nanika_localization::ResourceSet::new(
        nanika_localization::SupportedLocale::English,
        [(
            nanika_localization::SupportedLocale::English,
            vec![(
                "en.ftl".to_owned(),
                "candidate-kind = Application".to_owned()
            )]
        )]
        .into(),
        false
    )
    .unwrap();
    let values =
        nanika_localization::candidate_search_text(&candidate, Some(&resources), "macos").unwrap();
    let candidates = vec![nanika_search::Candidate::new(
        nanika_search::CandidateKind::Action,
        "application",
        &candidate.entry_id,
        "NetEaseMusic",
        "open",
        Vec::new(),
        values
    )];
    let mut search = nanika_search::SearchEngine::new();
    for _ in 0..10 {
        for (locale, title) in [
            (
                nanika_localization::SupportedLocale::English,
                "NetEaseMusic"
            ),
            (
                nanika_localization::SupportedLocale::SimplifiedChinese,
                "网易云音乐"
            )
        ] {
            assert_eq!(
                nanika_localization::format_text(
                    &candidate.title,
                    None,
                    locale,
                    "macos",
                    128,
                    false
                )
                .unwrap(),
                title
            );
            for query in [
                "网易",
                "音乐",
                "wy",
                "yy",
                "netease",
                "music",
                "網易",
                "ミュージック",
                "musique"
            ] {
                assert_eq!(
                    search
                        .query(query, &candidates, &nanika_search::UsageMap::new(), 0)
                        .results
                        .len(),
                    1,
                    "{query}"
                );
            }
        }
    }
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn scanning_rereads_added_changed_and_removed_localizations() {
    let root = _root("updates");
    _strings(&root, "en", "Music");
    _strings(&root, "zh-Hant", "音樂");
    assert_eq!(
        _names(&root, "Fallback", None),
        [
            ("en".to_owned(), "Music".to_owned()),
            ("zh-Hant".to_owned(), "音樂".to_owned())
        ]
        .into()
    );
    _strings(&root, "zh_CN", "音乐");
    assert_eq!(_names(&root, "Fallback", None)["zh-CN"], "音乐");
    _strings(&root, "zh_CN", "新音乐");
    assert_eq!(_names(&root, "Fallback", None)["zh-CN"], "新音乐");
    std::fs::remove_dir_all(root.join("Contents/Resources/zh_CN.lproj")).unwrap();
    assert!(!_names(&root, "Fallback", None).contains_key("zh-CN"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn names_support_loctable_binary_and_utf16_strings_with_bounded_input() {
    let root = _root("formats");
    _strings(&root, "en", "Fallback");
    let resources = root.join("Contents/Resources");
    let table = plist::Value::Dictionary(
        [(
            "en".to_owned(),
            plist::Value::Dictionary(
                [(
                    "CFBundleDisplayName".to_owned(),
                    plist::Value::String("Table Music".to_owned())
                )]
                .into_iter()
                .collect()
            )
        )]
        .into_iter()
        .collect()
    );
    table
        .to_file_binary(resources.join("InfoPlist.loctable"))
        .unwrap();
    _strings(
        &root,
        "en-aaaaa-bbbbb-ccccc-ddddd-eeeee-fffff-ggggg-hhhhh-iiiii-jjjjj-kkkkk",
        "Invalid locale must not remove other names"
    );
    let chinese = resources.join("zh-Hans.lproj/InfoPlist.strings");
    std::fs::create_dir_all(chinese.parent().unwrap()).unwrap();
    let bytes: Vec<u8> = [0xFEFF_u16]
        .into_iter()
        .chain("\"CFBundleDisplayName\" = \"网易云音乐\";".encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    std::fs::write(&chinese, bytes).unwrap();
    assert_eq!(
        _names(&root, "Fallback", None),
        [
            ("en".to_owned(), "Table Music".to_owned()),
            ("zh-Hans".to_owned(), "网易云音乐".to_owned())
        ]
        .into()
    );
    std::fs::write(&chinese, vec![b' '; MAX_METADATA_BYTES as usize + 1]).unwrap();
    assert!(!_names(&root, "Fallback", None).contains_key("zh-Hans"));
    assert_eq!(_names(&root, "Fallback", None)["en"], "Table Music");
    std::fs::remove_dir_all(root).unwrap();
}

fn _strings(root: &Path, locale: &str, name: &str) {
    let path = root.join(format!(
        "Contents/Resources/{locale}.lproj/InfoPlist.strings"
    ));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("\"CFBundleDisplayName\" = \"{name}\";")).unwrap();
}

fn _root(name: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("nanika-native-names-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn declared_source_language_remains_searchable_alongside_supplied_english() {
    let root = _root("source-language");
    _strings(&root, "en", "Music");
    assert_eq!(
        _names(&root, "音乐", Some("zh-Hans")),
        [
            ("en".to_owned(), "Music".to_owned()),
            ("zh-Hans".to_owned(), "音乐".to_owned())
        ]
        .into()
    );
    assert!(!_names(&root, "音乐", None).contains_key("zh-Hans"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn base_names_and_every_language_variant_survive_resource_fallback() {
    let root = _root("all-locales");
    _strings(&root, "Base", "Source Music");
    _strings(&root, "en_US", "Regional Music");
    _strings(&root, "zh_TW", "網易雲音樂");
    _strings(&root, "Japanese", "ミュージック");
    std::fs::create_dir_all(root.join("Contents/Resources/zh-Hans.lproj")).unwrap();
    let (fallback, names) = read(&root, "Internal", Some("en"));
    assert_eq!(fallback, "Source Music");
    assert!(!names.contains_key("en"));
    assert_eq!(names["en-US"], "Regional Music");
    assert_eq!(names["zh-TW"], "網易雲音樂");
    assert_eq!(names["ja"], "ミュージック");
    let text = nanika_protocol::TranslatedText {
        fallback,
        translations: names
    };
    text.validate().unwrap();
    assert_eq!(text.resolve("zh-Hans"), "網易雲音樂");
    assert_eq!(text.resolve("ja"), "ミュージック");
    assert_eq!(text.resolve("en"), "Regional Music");
    _strings(&root, "zh_CN", "网易云音乐");
    let (fallback, translations) = read(&root, "Internal", Some("en"));
    assert_eq!(
        nanika_protocol::TranslatedText {
            fallback,
            translations
        }
        .resolve("zh-Hans"),
        "网易云音乐"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn many_source_locales_are_preserved_without_a_ui_language_count_limit() {
    let root = _root("many-locales");
    for index in 1..=64 {
        _strings(
            &root,
            &format!("en-var{index:04}"),
            &format!("Music {index}")
        );
    }
    let (fallback, translations) = read(&root, "Music", None);
    assert_eq!(translations.len(), 64);
    nanika_protocol::TranslatedText {
        fallback,
        translations
    }
    .validate()
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn special_name_files_are_rejected_without_blocking_open() {
    let root = _root("fifo");
    let fifo = root.join("InfoPlist.strings");
    let path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let link = root.join("linked.strings");
    std::os::unix::fs::symlink(&fifo, &link).unwrap();
    for input in [fifo.clone(), link] {
        let (sender, receiver) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            sender.send(_read(&input).is_err()).unwrap();
        });
        let result = receiver.recv_timeout(std::time::Duration::from_secs(2));
        if result.is_err() {
            // Unblock a regressed reader before joining, so a failed assertion cannot strand a thread.
            drop(
                std::fs::OpenOptions::new()
                    .write(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(&fifo)
                    .unwrap()
            );
        }
        reader.join().unwrap();
        assert!(result.unwrap());
    }
    std::fs::remove_dir_all(root).unwrap();
}

fn _names(path: &Path, fallback: &str, locale: Option<&str>) -> BTreeMap<String, String> {
    read(path, fallback, locale).1
}

#[test]
fn default_names_do_not_shadow_regional_names_and_legacy_aliases_are_normalized() {
    let root = _root("source-fallback-priority");
    _strings(&root, "en_US", "Localized Music");
    let (fallback, translations) = read(&root, "Internal", Some("en"));
    let text = nanika_protocol::TranslatedText {
        fallback,
        translations
    };
    text.validate().unwrap();
    assert_eq!(text.resolve("en"), "Localized Music");
    assert!(text.values().any(|name| name == "Internal"));
    _strings(&root, "eng", "English Music");
    let (fallback, translations) = read(&root, "Internal", Some("en"));
    assert_eq!(translations["en"], "English Music");
    assert!(!translations.contains_key("eng"));
    let text = nanika_protocol::TranslatedText {
        fallback,
        translations
    };
    text.validate().unwrap();
    assert_eq!(text.resolve("en"), "English Music");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_optional_names_preserve_the_complete_scan_cache_and_search() {
    let root = _root("invalid-defaults");
    let applications = root.join("applications");
    let oversized = "x".repeat(16_385);
    let cases = [
        (Some(""), Some("Valid Source"), None, "Valid Source"),
        (
            Some(""),
            Some(""),
            Some(("en-GB", "Regional Music")),
            "Regional Music"
        ),
        (Some(" \t\n "), None, Some(("ja", "音楽箱")), "音楽箱"),
        (
            Some("bad\0name"),
            Some("bad\0name"),
            Some(("zh-Hant", "音樂")),
            "音樂"
        ),
        (
            Some("Valid Display"),
            Some(oversized.as_str()),
            None,
            "Valid Display"
        ),
        (Some(oversized.as_str()), None, None, "Fallback5"),
        (None, None, None, "Fallback6"),
        (Some("  Trimmed.APP  "), Some("Alias"), None, "Trimmed")
    ];
    for (index, (display, source, translation, _)) in cases.iter().enumerate() {
        let bundle = applications.join(format!("Fallback{index}.app"));
        let contents = bundle.join("Contents");
        std::fs::create_dir_all(contents.join("MacOS")).unwrap();
        let executable = contents.join("MacOS/Music");
        std::fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut info = plist::Dictionary::new();
        info.insert("CFBundleExecutable".into(), "Music".into());
        info.insert("CFBundleDevelopmentRegion".into(), "en".into());
        for (key, value) in [("CFBundleDisplayName", display), ("CFBundleName", source)] {
            if let Some(value) = value {
                info.insert(key.into(), (*value).into());
            }
        }
        plist::Value::Dictionary(info)
            .to_file_binary(contents.join("Info.plist"))
            .unwrap();
        if let Some((locale, name)) = translation {
            _strings(&bundle, locale, name);
        }
    }
    let database_path = root.join("applications.db");
    let mut index =
        crate::ApplicationIndex::new(crate::ApplicationDatabase::open(&database_path).unwrap());
    let config = crate::ApplicationConfig {
        roots: vec![applications],
        exclusions: Vec::new(),
        enabled_builtin_sources: Default::default()
    };
    for generation in 1..=2 {
        let report = index
            .scan(
                &config,
                generation,
                &std::sync::atomic::AtomicU64::new(0),
                |_| {},
                |_, _| {}
            )
            .unwrap();
        assert!(report.complete);
        assert_eq!(report.warnings, 0);
        assert_eq!(index.load().unwrap().len(), cases.len());
    }
    drop(index);
    let database = crate::ApplicationDatabase::open(&database_path).unwrap();
    let entries = database.load_entries().unwrap();
    assert_eq!(entries.len(), cases.len());
    for (number, (_, _, _, expected)) in cases.iter().enumerate() {
        let entry = entries
            .iter()
            .find(|entry| entry.source_key.ends_with(&format!("Fallback{number}.app")))
            .unwrap();
        assert_eq!(&entry.display_name, expected);
        let mut candidate = entry.candidate();
        candidate.subtitle = None;
        let values = nanika_localization::candidate_search_text(&candidate, None, "macos").unwrap();
        assert!(values.iter().any(|value| value == expected));
        assert!(values.iter().all(|value| value.len() <= 1024));
        for locale in [
            nanika_localization::SupportedLocale::English,
            nanika_localization::SupportedLocale::SimplifiedChinese
        ] {
            assert_eq!(
                nanika_localization::format_text(
                    &candidate.title,
                    None,
                    locale,
                    "macos",
                    1024,
                    false
                )
                .unwrap(),
                *expected
            );
        }
    }
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn filename_fallback_is_bounded_and_does_not_invent_a_source_language() {
    let root = _root("filename-fallback");
    for filename in ["Music.app", ".app", "\n.app"] {
        let path = root.join(filename);
        let (fallback, translations) = read(&path, "", Some("en"));
        assert!(translations.is_empty());
        assert!(!fallback.trim().is_empty());
        assert!(fallback.len() <= 1024);
        assert!(!fallback.chars().any(char::is_control));
    }
    _strings(&root, "ja", "音楽箱");
    let (fallback, translations) = read(&root, "", Some("en"));
    assert_eq!(fallback, "音楽箱");
    assert!(!translations.contains_key("en"));
    std::fs::remove_dir_all(root).unwrap();
}
