use super::*;
use crate::ApplicationEntryData;
use crate::ScanReport;

fn entry(id: &str) -> ApplicationEntry {
    ApplicationEntry::new(ApplicationEntryData {
        entry_id: id.to_owned(),
        source_key: id.to_owned(),
        display_name: id.to_owned(),
        normalized_name: id.to_owned(),
        normalized_tokens: id.to_owned(),
        launch_kind: "macos-bundle".to_owned(),
        target_path: format!("/{id}.app"),
        arguments_json: "{\"kind\":\"structured\",\"values\":[]}".to_owned(),
        icon_key: id.to_owned(),
        icon_source: None,
        icon_index: 0,
        priority: 0,
    })
}

#[test]
fn icon_preparation_only_consumes_requested_entries() {
    let root =
        std::env::temp_dir().join(format!("nanika-application-demand-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(root.join("index.db")).unwrap(),
        IconCache::new(root.join("icons")),
    );
    index.prepared_entries = Some(
        [entry("a"), entry("b"), entry("c")]
            .into_iter()
            .map(|entry| (entry.entry_id.clone(), entry))
            .collect(),
    );
    index.pending_icons = ["a", "b", "c"].map(str::to_owned).into_iter().collect();
    let (_, updated) = index.populate_icon_batch(&AtomicU64::new(0), 1, 10, &["b".into()]);
    assert!(!index.has_pending_icons_for(&["b".into()]));
    assert!(index.has_pending_icons_for(&["a".into()]));
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].entry_id, "b");
    assert_eq!(index.pending_icons, HashSet::from(["a".into(), "c".into()]));
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}

fn scan(
    index: &mut ApplicationIndex,
    config: &ApplicationConfig,
    generation: u64,
    cancelled: &AtomicU64,
    progress: impl FnMut(nanika_protocol::OperationProgress),
    mut publish: impl FnMut(Vec<ApplicationEntry>),
) -> Result<(ScanReport, Vec<ApplicationEntry>), ApplicationError> {
    let mut visible = index
        .load()?
        .into_iter()
        .map(|entry| (entry.entry_id.clone(), entry))
        .collect::<HashMap<_, _>>();
    let report = index.scan(
        config,
        generation,
        cancelled,
        progress,
        |updated, removed| {
            for id in removed {
                visible.remove(&id);
            }
            for entry in updated {
                visible.insert(entry.entry_id.clone(), entry);
            }
            publish(visible.values().cloned().collect());
        },
    )?;
    Ok((report, index.load()?))
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkDataList, IShellLinkW, SLDF_RUNAS_USER, ShellLink};
    use windows::Win32::UI::WindowsAndMessaging::{
        SHOW_WINDOW_CMD, SW_SHOWMAXIMIZED, SW_SHOWNORMAL,
    };
    use windows::core::{Interface, PCWSTR};

    use super::scan;
    use crate::platform;
    use crate::{
        ApplicationConfig, ApplicationDatabase, ApplicationIndex, DiscoveryState, IconCache,
    };

    #[test]
    fn configured_windows_root_is_discovered_and_persisted() {
        let root = test_root("configured-root");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).expect("application root should exist");
        create_executable(&applications.join("Sample Tool.exe"));
        let database =
            ApplicationDatabase::open(root.join("application.db")).expect("database should open");
        let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: platform::standard_roots().expect("standard roots"),
            enabled_builtin_roots: Default::default(),
        };
        let (report, entries) = scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {})
            .expect("scan should complete");
        assert!(report.complete);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display_name, "Sample Tool");
        assert_eq!(entries[0].launch_kind, "executable");
        assert_eq!(index.load().expect("persisted entries").len(), 1);
        drop(index);
        std::fs::remove_dir_all(root).expect("test root should be removable");
    }

    #[test]
    fn partial_scan_cleans_successful_paths_and_removed_roots_but_preserves_failed_paths() {
        let root = test_root("independent-roots");
        let valid = root.join("valid");
        let broken = valid.join("broken-package");
        let removed = root.join("removed");
        std::fs::create_dir_all(&valid).unwrap();
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::create_dir_all(&removed).unwrap();
        create_executable(&removed.join("Removed.exe"));
        std::fs::write(broken.join("AppxManifest.xml"),
            r#"<Package><Identity Name="Retained"/><Application Id="App" DisplayName="Retained"/></Package>"#).unwrap();
        create_executable(&valid.join("Previous.exe"));
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(root.join("application.db")).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let mut config = ApplicationConfig {
            roots: vec![valid.clone(), removed],
            exclusions: Vec::new(),
            enabled_builtin_roots: Default::default(),
        };
        assert!(
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {})
                .unwrap()
                .0
                .complete
        );
        std::fs::remove_file(valid.join("Previous.exe")).unwrap();
        create_executable(&valid.join("Current.exe"));
        std::fs::write(broken.join("AppxManifest.xml"), "invalid manifest").unwrap();
        config.roots.pop();
        let (report, entries) =
            scan(&mut index, &config, 2, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(!report.complete);
        assert_eq!(report.warnings, 1);
        assert!(entries.iter().any(|entry| entry.display_name == "Current"));
        assert!(entries.iter().any(|entry| entry.display_name == "Retained"));
        assert!(!entries.iter().any(|entry| entry.display_name == "Previous"));
        assert!(!entries.iter().any(|entry| entry.display_name == "Removed"));
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn completed_root_is_published_and_persisted_before_the_next_root_starts() {
        use std::sync::atomic::Ordering;
        let root = test_root("root-commit");
        let first = root.join("a-first");
        let second = root.join("b-second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(second.join("nested")).unwrap();
        create_executable(&first.join("OldFirst.exe"));
        create_executable(&second.join("nested/OldSecond.exe"));
        let db_path = root.join("application.db");
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(&db_path).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let config = ApplicationConfig {
            roots: vec![first.clone(), second.clone()],
            exclusions: Vec::new(),
            enabled_builtin_roots: Default::default(),
        };
        scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        std::fs::remove_file(first.join("OldFirst.exe")).unwrap();
        std::fs::remove_file(second.join("nested/OldSecond.exe")).unwrap();
        create_executable(&first.join("NewFirst.exe"));
        create_executable(&second.join("nested/NewSecond.exe"));
        let cancelled = AtomicU64::new(0);
        let observer = rusqlite::Connection::open(&db_path).unwrap();
        let mut publications = 0;
        let (report, entries) =
            scan(
                &mut index,
                &config,
                2,
                &cancelled,
                |_| {},
                |entries| {
                    publications += 1;
                    assert!(entries.iter().any(|entry| entry.display_name == "NewFirst"));
                    assert!(
                        entries
                            .iter()
                            .any(|entry| entry.display_name == "OldSecond")
                    );
                    assert!(!entries.iter().any(|entry| entry.display_name == "OldFirst"
                        || entry.display_name == "NewSecond"));
                    let persisted: i64 = observer
                        .query_row(
                            "SELECT count(*) FROM app_sources WHERE display_name = 'NewFirst'",
                            [],
                            |row| row.get(0),
                        )
                        .unwrap();
                    assert_eq!(persisted, 1);
                    cancelled.store(2, Ordering::Release);
                },
            )
            .unwrap();
        assert!(report.cancelled);
        assert_eq!(publications, 1);
        assert_eq!(entries.len(), 2);
        drop(observer);
        drop(index);
        let mut restarted = ApplicationIndex::new(
            ApplicationDatabase::open(&db_path).unwrap(),
            IconCache::new(root.join("icons")),
        );
        assert!(
            restarted
                .load()
                .unwrap()
                .iter()
                .any(|entry| entry.display_name == "OldSecond")
        );
        let (_, entries) = scan(
            &mut restarted,
            &config,
            3,
            &AtomicU64::new(0),
            |_| {},
            |_| {},
        )
        .unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.display_name == "NewSecond")
        );
        assert!(
            !entries
                .iter()
                .any(|entry| entry.display_name.starts_with("Old"))
        );
        drop(restarted);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unchanged_roots_do_not_write_or_publish_and_one_deletion_is_a_small_patch() {
        let root = test_root("root-delta");
        let first = root.join("a");
        let second = root.join("b");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        create_executable(&first.join("Remove.exe"));
        create_executable(&second.join("Keep.exe"));
        let path = root.join("application.db");
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(&path).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let config = ApplicationConfig {
            roots: vec![first.clone(), second],
            exclusions: Vec::new(),
            enabled_builtin_roots: Default::default(),
        };
        index
            .scan(&config, 1, &AtomicU64::new(0), |_| {}, |_, _| {})
            .unwrap();
        let observer = rusqlite::Connection::open(&path).unwrap();
        observer.execute_batch("CREATE TABLE audit(kind TEXT); CREATE TRIGGER track_insert AFTER INSERT ON app_sources BEGIN INSERT INTO audit VALUES ('insert'); END; CREATE TRIGGER track_update AFTER UPDATE ON app_sources BEGIN INSERT INTO audit VALUES ('update'); END; CREATE TRIGGER track_delete AFTER DELETE ON app_sources BEGIN INSERT INTO audit VALUES ('delete'); END;").unwrap();
        let before: i64 = observer
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .unwrap();
        index
            .scan(
                &config,
                2,
                &AtomicU64::new(0),
                |_| {},
                |_, _| panic!("unchanged roots must not publish"),
            )
            .unwrap();
        let count: i64 = observer
            .query_row("SELECT count(*) FROM audit", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(
            observer
                .query_row("PRAGMA data_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            before,
            "unchanged scans must not commit any database writes"
        );
        std::fs::remove_file(first.join("Remove.exe")).unwrap();
        let mut patches = 0;
        index
            .scan(
                &config,
                3,
                &AtomicU64::new(0),
                |_| {},
                |updated, removed| {
                    patches += 1;
                    assert!(updated.is_empty());
                    assert_eq!(removed.len(), 1);
                },
            )
            .unwrap();
        assert_eq!(patches, 1);
        let kinds = observer
            .prepare("SELECT kind FROM audit")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(kinds, ["delete"]);
        drop(observer);
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancellation_does_not_stale_the_previous_snapshot() {
        let root = test_root("cancellation");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).expect("application root should exist");
        create_executable(&applications.join("First.exe"));
        let database =
            ApplicationDatabase::open(root.join("application.db")).expect("database should open");
        let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
        let config = ApplicationConfig {
            roots: vec![applications.clone()],
            exclusions: platform::standard_roots().expect("standard roots"),
            enabled_builtin_roots: Default::default(),
        };
        scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {})
            .expect("first scan should complete");
        std::fs::remove_file(applications.join("First.exe"))
            .expect("test executable should remove");
        let (report, entries) = scan(&mut index, &config, 2, &AtomicU64::new(2), |_| {}, |_| {})
            .expect("cancelled scan should commit its state");
        assert!(report.cancelled);
        assert_eq!(entries.len(), 1);
        drop(index);
        std::fs::remove_dir_all(root).expect("test root should be removable");
    }

    #[test]
    fn deleted_sources_are_removed_and_other_roots_remain_searchable() {
        let root = test_root("deleted-sources");
        let removed = root.join("removed");
        let retained = root.join("retained");
        std::fs::create_dir_all(&removed).unwrap();
        std::fs::create_dir_all(&retained).unwrap();
        create_executable(&removed.join("Deleted.exe"));
        create_executable(&retained.join("Keep.exe"));
        let shortcut = retained.join("Temporary Link.lnk");
        let shortcut_target = root.join("ShortcutTarget.exe");
        create_executable(&shortcut_target);
        create_shell_link(&shortcut, &shortcut_target);
        let database = ApplicationDatabase::open(root.join("application.db")).unwrap();
        let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
        let config = ApplicationConfig {
            roots: vec![removed.clone(), retained.clone()],
            exclusions: platform::standard_roots().unwrap(),
            enabled_builtin_roots: Default::default(),
        };
        let (report, initial) =
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        assert_eq!(initial.len(), 3);
        std::fs::remove_file(shortcut).unwrap();
        std::fs::remove_file(removed.join("Deleted.exe")).unwrap();
        std::fs::remove_dir(&removed).unwrap();
        let (report, remaining) =
            scan(&mut index, &config, 2, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        assert_eq!(report.warnings, 0);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].display_name, "Keep");
        assert_eq!(index.load().unwrap().len(), 1);
        assert!(!removed.exists());
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn standard_windows_roots_produce_valid_application_metadata() {
        let root = test_root("standard-roots");
        let database =
            ApplicationDatabase::open(root.join("application.db")).expect("database should open");
        let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
        let config = ApplicationConfig {
            roots: Vec::new(),
            exclusions: Vec::new(),
            enabled_builtin_roots: serde_json::from_str::<serde_json::Value>(include_str!(
                "../manifest.jsonc"
            ))
            .unwrap()["contributes"]["configuration"]["properties"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, property)| property["default"] == true)
                .map(|(key, _)| key.clone())
                .collect(),
        };
        let (report, entries) = scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {})
            .expect("standard application scan should complete");
        assert!(!report.cancelled);
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|entry| {
            !entry.entry_id.is_empty()
                && !entry.display_name.is_empty()
                && PathBuf::from(&entry.target_path).is_file()
        }));
        // Explicit folders remain eligible after their built-in discovery source is disabled.
        let retained = entries
            .iter()
            .find(|entry| {
                entry.launch_kind == "windows-shell-link" || entry.launch_kind == "executable"
            })
            .unwrap();
        let mut disabled = config.clone();
        disabled.enabled_builtin_roots.clear();
        disabled.roots = vec![PathBuf::from(&retained.target_path)];
        let (report, remaining) =
            scan(&mut index, &disabled, 2, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].entry_id, retained.entry_id);
        disabled.roots.clear();
        let (report, empty) =
            scan(&mut index, &disabled, 3, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        assert!(empty.is_empty());
        drop(index);
        std::fs::remove_dir_all(root).expect("test root should be removable");
    }

    #[test]
    fn argument_free_shortcuts_deduplicate_with_their_direct_executable() {
        let root = test_root("shortcut-identity");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).expect("application root should exist");
        let executable = applications.join("Sample.exe");
        let shortcut = applications.join("Sample.lnk");
        create_executable(&executable);
        create_shell_link(&shortcut, &executable);
        let mut discovery_state = DiscoveryState::new();
        let direct_entry = platform::read_entry(&mut discovery_state, &executable, 1)
            .expect("direct executable should parse")
            .expect("direct executable should contribute an entry");
        let shortcut_entry = platform::read_entry(&mut discovery_state, &shortcut, 1)
            .expect("shortcut should parse")
            .expect("shortcut should contribute an entry");
        assert_eq!(shortcut_entry.entry_id, direct_entry.entry_id);
        assert_eq!(
            shortcut_entry.launch_descriptor().unwrap(),
            nanika_protocol::LaunchDescriptor::WindowsApplication {
                path: shortcut.to_string_lossy().into_owned(),
            }
        );
        assert!(matches!(
            direct_entry.launch_descriptor().unwrap(),
            nanika_protocol::LaunchDescriptor::WindowsApplication { .. }
        ));

        let database =
            ApplicationDatabase::open(root.join("application.db")).expect("database should open");
        let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: platform::standard_roots().expect("standard roots"),
            enabled_builtin_roots: Default::default(),
        };

        let (_, entries) = scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {})
            .expect("application scan should complete");

        assert_eq!(entries.len(), 1);
        drop(index);
        std::fs::remove_dir_all(root).expect("test root should be removable");
    }

    #[test]
    fn shortcut_icons_preserve_the_configured_resource_index() {
        let root = test_root("shortcut-icons");
        let executable = root.join("Sample.exe");
        create_executable(&executable);
        let resource = PathBuf::from(std::env::var_os("WINDIR").unwrap())
            .join("System32")
            .join("shell32.dll");
        let cache = IconCache::new(root.join("icons"));
        let mut images = Vec::new();
        let mut state = DiscoveryState::new();
        for index in [0, 3] {
            let shortcut = root.join(format!("Icon {index}.lnk"));
            create_shell_link_configured(
                &shortcut,
                &executable,
                Some((&resource, index)),
                false,
                SW_SHOWNORMAL,
                None,
            );
            let mut entry = platform::read_entry(&mut state, &shortcut, 0)
                .unwrap()
                .unwrap();
            cache.prepare(&mut entry).unwrap();
            images.push(
                std::fs::read(root.join("icons").join(&entry.icon_key).join("128.png")).unwrap(),
            );
        }
        assert_ne!(
            images[0], images[1],
            "different shortcut icons must not become the DLL's generic icon"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn overlapping_roots_and_case_variants_publish_one_native_entry() {
        let root = test_root("overlapping-roots");
        let applications = root.join("applications");
        let nested = applications.join("nested");
        let shortcuts = root.join("aliases");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&shortcuts).unwrap();
        let executable = nested.join("Tool.exe");
        let shortcut = shortcuts.join("Preferred Tool.lnk");
        create_executable(&executable);
        create_shell_link_configured(&shortcut, &executable, None, false, SW_SHOWMAXIMIZED, None);
        let config = ApplicationConfig {
            roots: vec![
                applications.clone(),
                nested,
                shortcuts,
                PathBuf::from(applications.to_string_lossy().to_uppercase()),
            ],
            exclusions: platform::standard_roots().unwrap(),
            enabled_builtin_roots: Default::default(),
        };
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(root.join("application.db")).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let (report, entries) =
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display_name, "Preferred Tool");
        assert_eq!(entries[0].target_path, shortcut.to_string_lossy());
        assert_eq!(index.load().unwrap().len(), 1);
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shortcut_launch_path_preserves_unicode_after_database_reload() {
        let root = test_root("unicode-launch-path");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).unwrap();
        let executable = root.join("Sample.exe");
        create_executable(&executable);
        let shortcut = applications.join("İstanbul.lnk");
        create_shell_link(&shortcut, &executable);
        let database_path = root.join("application.db");
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: ApplicationConfig::standard_roots().unwrap(),
            enabled_builtin_roots: Default::default(),
        };
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(&database_path).unwrap(),
            IconCache::new(root.join("icons")),
        );
        assert!(
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {})
                .unwrap()
                .0
                .complete
        );
        drop(index);
        let database = ApplicationDatabase::open(&database_path).unwrap();
        let entries = database.load_entries().unwrap();
        assert_eq!(entries.len(), 1);
        let nanika_protocol::LaunchDescriptor::WindowsApplication { path } =
            entries[0].launch_descriptor().unwrap()
        else {
            panic!("shortcut should use native application activation");
        };
        assert_eq!(PathBuf::from(&path), shortcut);
        assert!(PathBuf::from(path).is_file());
        assert_ne!(entries[0].source_key, entries[0].target_path);
        drop(database);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pre_epoch_shortcut_and_icon_timestamps_do_not_prevent_discovery() {
        let root = test_root("pre-epoch-icons");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).unwrap();
        let executable = root.join("Sample.exe");
        // Copy so changing the timestamp cannot affect the running test executable.
        std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        let shortcut = applications.join("OldStamp.lnk");
        create_shell_link(&shortcut, &executable);
        let old_time = std::time::UNIX_EPOCH - std::time::Duration::from_secs(2_208_988_800);
        for path in [&shortcut, &executable] {
            std::fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(old_time)
                .unwrap();
        }
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: ApplicationConfig::standard_roots().unwrap(),
            enabled_builtin_roots: Default::default(),
        };
        let database_path = root.join("application.db");
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(&database_path).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let (report, entries) =
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        assert_eq!(report.warnings, 0);
        assert_eq!(entries.len(), 1);
        let original_key = index.load().unwrap()[0].icon_key.clone();
        assert_ne!(original_key, IconCache::fallback_key());
        std::fs::File::options()
            .write(true)
            .open(&executable)
            .unwrap()
            .set_modified(old_time + std::time::Duration::from_secs(1))
            .unwrap();
        assert!(
            scan(&mut index, &config, 2, &AtomicU64::new(0), |_| {}, |_| {})
                .unwrap()
                .0
                .complete
        );
        assert_ne!(index.load().unwrap()[0].icon_key, original_key);
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shortcut_activation_variants_share_identity_and_retain_native_launch_paths() {
        let root = test_root("shortcut-activation");
        let executable = root.join("Sample.exe");
        create_executable(&executable);
        let mut state = DiscoveryState::new();
        let direct = platform::read_entry(&mut state, &executable, 0)
            .unwrap()
            .unwrap();
        for (name, elevated, show_command) in [
            ("Elevated", true, SW_SHOWNORMAL),
            ("Maximized", false, SW_SHOWMAXIMIZED),
        ] {
            let shortcut = root.join(format!("{name}.lnk"));
            create_shell_link_configured(
                &shortcut,
                &executable,
                None,
                elevated,
                show_command,
                None,
            );
            let entry = platform::read_entry(&mut state, &shortcut, 0)
                .unwrap()
                .unwrap();
            assert_eq!(entry.entry_id, direct.entry_id);
            assert_eq!(
                entry.launch_descriptor().unwrap(),
                nanika_protocol::LaunchDescriptor::WindowsApplication {
                    path: shortcut.to_string_lossy().into_owned(),
                }
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shortcuts_with_different_arguments_remain_separate() {
        let root = test_root("shortcut-arguments");
        let executable = root.join("Sample.exe");
        create_executable(&executable);
        let mut state = DiscoveryState::new();
        let direct = platform::read_entry(&mut state, &executable, 0)
            .unwrap()
            .unwrap();
        let mut ids = std::collections::HashSet::from([direct.entry_id.clone()]);
        for (name, arguments) in [
            ("Work", "--profile work"),
            ("Personal", "--profile personal"),
        ] {
            let shortcut = root.join(format!("{name}.lnk"));
            create_shell_link_configured(
                &shortcut,
                &executable,
                None,
                false,
                SW_SHOWNORMAL,
                Some(arguments),
            );
            let entry = platform::read_entry(&mut state, &shortcut, 0)
                .unwrap()
                .unwrap();
            assert!(ids.insert(entry.entry_id.clone()));
            assert_eq!(
                entry.arguments_json,
                crate::ApplicationArguments::from_windows_raw(Some(arguments.to_owned()))
                    .to_json()
                    .unwrap()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_windows_executables_are_rejected() {
        let root = test_root("invalid-executable");
        let executable = root.join("Invalid.exe");
        std::fs::write(&executable, []).expect("invalid executable should exist");

        assert!(
            platform::read_entry(&mut DiscoveryState::new(), &executable, 0)
                .expect("invalid executable should not produce an I/O error")
                .is_none()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn executable_validation_cache_rechecks_changed_files() {
        let root = test_root("changed-executable");
        let executable = root.join("Changed.exe");
        create_executable(&executable);
        let mut discovery_state = DiscoveryState::new();
        assert!(
            platform::read_entry(&mut discovery_state, &executable, 0)
                .expect("valid executable should parse")
                .is_some()
        );
        std::fs::remove_file(&executable).expect("valid executable should remove");
        std::fs::write(&executable, []).expect("invalid executable should replace it");
        discovery_state.begin_scan();

        assert!(
            platform::read_entry(&mut discovery_state, &executable, 0)
                .expect("changed executable should parse")
                .is_none()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_index_transactions_do_not_publish_uncommitted_records() {
        let root = test_root("failed-transaction");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).expect("application root should exist");
        create_executable(&applications.join("Sample.exe"));
        let database_path = root.join("application.db");
        let database = ApplicationDatabase::open(&database_path).expect("database should open");
        let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
        let observer = rusqlite::Connection::open(&database_path).expect("observer should open");
        observer
            .execute_batch(
                "CREATE TRIGGER reject_application_insert BEFORE INSERT ON app_sources BEGIN SELECT RAISE(ABORT, 'rejected by test'); END;",
            )
            .expect("failure trigger should install");
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: platform::standard_roots().expect("standard roots"),
            enabled_builtin_roots: Default::default(),
        };

        assert!(scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).is_err());
        assert!(index.load().unwrap().is_empty());
        assert_eq!(
            observer
                .query_row("SELECT count(*) FROM app_sources", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(observer);
        drop(index);
        std::fs::remove_dir_all(root).expect("test root should be removable");
    }

    #[test]
    fn scoop_shims_share_target_identity_but_preserve_arguments_and_native_activation() {
        let root = test_root("shim-identity");
        let shims = root.join("shims");
        let apps = root.join("apps");
        std::fs::create_dir_all(&shims).unwrap();
        std::fs::create_dir_all(&apps).unwrap();
        let target = apps.join("Sample App.exe");
        create_executable(&target);
        let shim = shims.join("alias.exe");
        create_executable(&shim);
        std::fs::write(
            shim.with_extension("shim"),
            format!("path = \"{}\"\r\n", target.display()),
        )
        .unwrap();
        let mut state = DiscoveryState::new();
        let direct = platform::read_entry(&mut state, &target, 0)
            .unwrap()
            .unwrap();
        let alias = platform::read_entry(&mut state, &shim, 0).unwrap().unwrap();
        assert_eq!(alias.entry_id, direct.entry_id);
        assert_eq!(
            alias.launch_descriptor().unwrap(),
            nanika_protocol::LaunchDescriptor::WindowsApplication {
                path: shim.canonicalize().unwrap().to_string_lossy().into_owned(),
            }
        );
        let link = root.join("shortcut.lnk");
        create_shell_link(&link, &target);
        assert_eq!(
            platform::read_entry(&mut state, &link, 0)
                .unwrap()
                .unwrap()
                .entry_id,
            alias.entry_id
        );
        create_shell_link(&link, &shim);
        assert_eq!(
            platform::read_entry(&mut state, &link, 0)
                .unwrap()
                .unwrap()
                .entry_id,
            alias.entry_id
        );

        std::fs::write(
            shim.with_extension("shim"),
            format!("path = \"{}\"\nargs = --profile work\n", target.display()),
        )
        .unwrap();
        let profile = platform::read_entry(&mut state, &shim, 0).unwrap().unwrap();
        assert_ne!(profile.entry_id, direct.entry_id);
        create_shell_link_configured(
            &link,
            &target,
            None,
            false,
            SW_SHOWNORMAL,
            Some("--profile work"),
        );
        assert_eq!(
            platform::read_entry(&mut state, &link, 0)
                .unwrap()
                .unwrap()
                .entry_id,
            profile.entry_id
        );

        let next = apps.join("Updated.exe");
        create_executable(&next);
        std::fs::write(
            shim.with_extension("shim"),
            format!("path = \"{}\"\n", next.display()),
        )
        .unwrap();
        let updated = platform::read_entry(&mut state, &shim, 0).unwrap().unwrap();
        assert_ne!(updated.entry_id, direct.entry_id);
        assert_eq!(
            updated.entry_id,
            platform::read_entry(&mut state, &next, 0)
                .unwrap()
                .unwrap()
                .entry_id
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_preferred_duplicate_keeps_its_metadata_until_removed() {
        let root = test_root("failed-preferred-duplicate");
        let applications = root.join("applications");
        std::fs::create_dir_all(&applications).unwrap();
        let target = root.join("Target.exe");
        create_executable(&target);
        let preferred = applications.join("A Preferred.lnk");
        create_shell_link(&preferred, &target);
        create_shell_link(&applications.join("Z Alternate.lnk"), &target);
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(root.join("application.db")).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: Vec::new(),
            enabled_builtin_roots: Default::default(),
        };
        let (_, initial) =
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].display_name, "A Preferred");
        std::fs::write(&preferred, "invalid shortcut").unwrap();
        let (report, retained) =
            scan(&mut index, &config, 2, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(!report.complete);
        assert_eq!(report.warnings, 1);
        std::fs::remove_file(&preferred).unwrap();
        let (report, promoted) =
            scan(&mut index, &config, 3, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert!(report.complete);
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(
            retained[0].display_name, "A Preferred",
            "a failed read must preserve the previous winner"
        );
        assert_eq!(promoted.len(), 1);
        assert_eq!(
            promoted[0].display_name, "Z Alternate",
            "deleting the winner must promote its remaining source"
        );
    }

    #[test]
    fn failed_scan_reconciles_icon_work_before_returning() {
        let root = test_root("failed-scan-icon-work");
        let first = root.join("a");
        let second = root.join("b");
        let third = root.join("c");
        for path in [&first, &second, &third] {
            std::fs::create_dir_all(path).unwrap();
        }
        let target = root.join("Shared.exe");
        create_executable(&target);
        let first_link = first.join("Shared.lnk");
        let second_link = second.join("Shared.lnk");
        create_shell_link(&first_link, &target);
        create_shell_link(&second_link, &target);
        let database = root.join("apps.db");
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(&database).unwrap(),
            IconCache::new(root.join("icons")),
        );
        let config = ApplicationConfig {
            roots: vec![first, second, third.clone()],
            exclusions: Vec::new(),
            enabled_builtin_roots: Default::default(),
        };
        let (_, entries) =
            scan(&mut index, &config, 1, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        assert_eq!(entries.len(), 1);
        std::fs::remove_file(first_link).unwrap();
        std::fs::remove_file(second_link).unwrap();
        create_executable(&third.join("New.exe"));
        let observer = rusqlite::Connection::open(&database).unwrap();
        observer.execute_batch("CREATE TRIGGER reject_insert BEFORE INSERT ON app_sources WHEN NEW.display_name = 'New' BEGIN SELECT RAISE(ABORT, 'injected'); END").unwrap();
        let result = scan(&mut index, &config, 2, &AtomicU64::new(0), |_| {}, |_| {});
        assert!(result.is_err());
        let pending_after_failure = index.has_pending_icons();
        let visible_after_failure = index.load().unwrap();
        observer
            .execute_batch("DROP TRIGGER reject_insert")
            .unwrap();
        let (report, recovered) =
            scan(&mut index, &config, 3, &AtomicU64::new(0), |_| {}, |_| {}).unwrap();
        drop(observer);
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
        assert!(
            !pending_after_failure,
            "scan must return with no icon work for removed identities"
        );
        assert!(
            visible_after_failure.is_empty(),
            "earlier root deletions must remain committed"
        );
        assert!(report.complete);
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].display_name, "New");
    }

    fn create_executable(target: &std::path::Path) {
        let source = std::env::current_exe().expect("test executable path");
        std::fs::hard_link(&source, target)
            .or_else(|_| std::fs::copy(&source, target).map(|_| ()))
            .expect("test executable should exist");
    }

    fn create_shell_link(path: &std::path::Path, target: &std::path::Path) {
        create_shell_link_configured(path, target, None, false, SW_SHOWNORMAL, None);
    }

    fn create_shell_link_configured(
        path: &std::path::Path,
        target: &std::path::Path,
        icon: Option<(&std::path::Path, i32)>,
        elevated: bool,
        show_command: SHOW_WINDOW_CMD,
        arguments: Option<&str>,
    ) {
        let initialization = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert!(initialization.is_ok() || initialization == RPC_E_CHANGED_MODE);
        let shell_link: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
                .expect("Shell Link should create");
        let target = target
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        unsafe {
            shell_link
                .SetPath(PCWSTR(target.as_ptr()))
                .expect("shortcut target should set");
        }
        if let Some((source, index)) = icon {
            let source = source
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            unsafe {
                shell_link
                    .SetIconLocation(PCWSTR(source.as_ptr()), index)
                    .unwrap()
            };
        }
        let persistence: IPersistFile = shell_link.cast().expect("persistence interface");
        if let Some(arguments) = arguments {
            let wide = arguments.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            unsafe { shell_link.SetArguments(PCWSTR(wide.as_ptr())).unwrap() };
        }
        unsafe { shell_link.SetShowCmd(show_command).unwrap() };
        if elevated {
            let data: IShellLinkDataList = shell_link.cast().unwrap();
            unsafe {
                data.SetFlags(data.GetFlags().unwrap() | SLDF_RUNAS_USER.0 as u32)
                    .unwrap()
            };
        }
        let path = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        unsafe {
            persistence
                .Save(PCWSTR(path.as_ptr()), true)
                .expect("shortcut should save");
        }
        drop(persistence);
        drop(shell_link);
        if initialization.is_ok() {
            unsafe {
                CoUninitialize();
            }
        }
    }

    fn test_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "nanika-application-index-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("test root should exist");
        root
    }
}

#[test]
fn committed_sources_preserve_winners_across_roots_failures_and_restart() {
    let root =
        std::env::temp_dir().join(format!("nanika-application-sources-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("apps.db");
    let low_root = root.join("a-low");
    let high_root = root.join("z-high");
    let low_key = path_key(&low_root);
    let high_key = path_key(&high_root);
    let mut low = entry("same-app");
    low.source_key = format!("{low_key}/app.exe");
    low.display_name = "Original".to_owned();
    low.normalized_name = "original".into();
    let mut high = low.clone();
    high.source_key = format!("{high_key}/app.exe");
    high.display_name = "Preferred".to_owned();
    high.normalized_name = "preferred".into();
    high.priority = 1;
    let map = |value: ApplicationEntry| HashMap::from([(value.entry_id.clone(), value)]);
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(&path).unwrap(),
        IconCache::new(root.join("icons")),
    );
    index.load().unwrap();
    index
        ._commit_root(low_key.clone(), map(low.clone()), &mut |_, _| {})
        .unwrap();
    index
        ._commit_root(high_key.clone(), map(high.clone()), &mut |_, _| {})
        .unwrap();
    // Reopening must seed the persisted winner before an earlier low-priority root is scanned.
    drop(index);
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(&path).unwrap(),
        IconCache::new(root.join("icons")),
    );
    index.load().unwrap();
    let observer = rusqlite::Connection::open(&path).unwrap();
    let version = || {
        observer
            .query_row("PRAGMA data_version", [], |row| row.get::<_, i64>(0))
            .unwrap()
    };
    let before = version();
    for _ in 0..2 {
        index
            ._commit_root(low_key.clone(), map(low.clone()), &mut |_, _| {
                panic!("losing sources do not publish")
            })
            .unwrap();
        index
            ._commit_root(high_key.clone(), map(high.clone()), &mut |_, _| {
                panic!("unchanged winner does not publish")
            })
            .unwrap();
    }
    assert_eq!(
        version(),
        before,
        "unchanged duplicate sources must not write"
    );
    observer.execute_batch("CREATE TRIGGER reject_change BEFORE DELETE ON app_sources BEGIN SELECT RAISE(ABORT, 'injected'); END").unwrap();
    assert!(
        index
            ._commit_root(high_key.clone(), HashMap::new(), &mut |_, _| panic!(
                "failed commit must not publish"
            ))
            .is_err()
    );
    assert_eq!(index.load().unwrap()[0].display_name, "Preferred");
    observer
        .execute_batch("DROP TRIGGER reject_change")
        .unwrap();
    let mut titles = Vec::new();
    index
        ._commit_root(high_key, HashMap::new(), &mut |updated, removed| {
            assert!(removed.is_empty());
            titles.extend(updated.into_iter().map(|entry| entry.display_name.clone()));
        })
        .unwrap();
    assert_eq!(titles, ["Original"]);
    drop(observer);
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}
