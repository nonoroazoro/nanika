use super::*;
use crate::ScanReport;

fn entry(id: &str) -> ApplicationEntry {
    ApplicationEntry {
        entry_id: id.to_owned(),
        source_key: id.to_owned(),
        display_name: id.to_owned(),
        normalized_name: id.to_owned(),
        normalized_tokens: id.to_owned(),
        search_readings: Vec::new(),
        launch_kind: "macos-bundle".to_owned(),
        target_path: format!("/{id}.app"),
        working_directory: None,
        arguments_json: "{\"kind\":\"structured\",\"values\":[]}".to_owned(),
        bundle_id: None,
        icon_key: id.to_owned(),
        icon_source: None,
        icon_index: 0,
        priority: 0,
    }
}

#[test]
fn host_visible_entries_move_to_the_front_in_host_order() {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-priority-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(root.join("index.db")).unwrap(),
        IconCache::new(root.join("icons")),
    );
    index.prepared_entries = Some(vec![entry("a"), entry("b"), entry("c"), entry("d")]);
    index.pending_icons = vec![0, 1, 2, 3];
    assert!(
        index
            .pending_icons
            .iter()
            .any(|&index_id| index.prepared_entries.as_ref().unwrap()[index_id].entry_id == "b")
    );
    assert!(
        !index
            .pending_icons
            .iter()
            .any(
                |&index_id| index.prepared_entries.as_ref().unwrap()[index_id].entry_id
                    == "missing"
            )
    );
    index.prioritize_pending_icons(&["d".to_owned(), "b".to_owned()]);
    assert_eq!(
        index
            .pending_icons
            .iter()
            .map(
                |&index_id| index.prepared_entries.as_ref().unwrap()[index_id]
                    .entry_id
                    .as_str()
            )
            .collect::<Vec<_>>(),
        ["d", "b", "a", "c"]
    );
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_names_reuse_readings_across_scans_and_icon_batches() {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-readings-cache-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut index = ApplicationIndex::new(
        ApplicationDatabase::open(root.join("index.db")).unwrap(),
        IconCache::new(root.join("icons")),
    );
    let mut application = entry("sync");
    application.display_name = "同步".to_owned();
    application.normalized_name = "同步".to_owned();
    application.normalized_tokens = "同步".to_owned();

    commit(&mut index, 1, &application);
    let initial = index.load_presentable().unwrap();
    assert_eq!(initial[0].search_readings[0].full, "tongbu");
    let allocation = index.prepared_entries.as_ref().unwrap()[0].search_readings[0]
        .full
        .as_ptr();

    commit(&mut index, 2, &application);
    index
        .cache_scanned_entries(vec![application.clone()])
        .unwrap();
    assert_eq!(
        index.prepared_entries.as_ref().unwrap()[0].search_readings[0]
            .full
            .as_ptr(),
        allocation,
        "unchanged search inputs must move cached readings, not rebuild them"
    );
    index.populate_icon_batch(&AtomicU64::new(0), 2, 1).unwrap();
    assert_eq!(
        index.prepared_entries.as_ref().unwrap()[0].search_readings[0]
            .full
            .as_ptr(),
        allocation,
        "icon-only work must not rebuild search readings"
    );
    assert_eq!(
        index.load_presentable().unwrap()[0].search_readings[0].full,
        "tongbu"
    );

    application.display_name = "音乐".to_owned();
    application.normalized_name = "音乐".to_owned();
    application.normalized_tokens = "音乐".to_owned();
    commit(&mut index, 3, &application);
    index.cache_scanned_entries(vec![application]).unwrap();
    assert_eq!(
        index.load_presentable().unwrap()[0].search_readings[0].full,
        "yinyue"
    );

    let mut application = entry("sync");
    application.display_name = "音乐".to_owned();
    application.normalized_name = "音乐".to_owned();
    application.normalized_tokens = "音乐\n同步".to_owned();
    commit(&mut index, 4, &application);
    index.cache_scanned_entries(vec![application]).unwrap();
    assert_eq!(
        index.load_presentable().unwrap()[0]
            .search_readings
            .iter()
            .map(|reading| reading.full.as_str())
            .collect::<Vec<_>>(),
        ["yinyue", "tongbu"],
        "alternate-name changes must invalidate the cached search inputs"
    );
    drop(index);
    std::fs::remove_dir_all(root).unwrap();
}

fn commit(index: &mut ApplicationIndex, generation: u64, application: &ApplicationEntry) {
    index.database.begin_scan(generation).unwrap();
    index
        .database
        .commit_scan(
            ScanReport {
                generation,
                discovered: 1,
                warnings: 0,
                complete: true,
                cancelled: false,
            },
            std::slice::from_ref(application),
            &[],
            None,
        )
        .unwrap();
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
        let (report, entries) = index
            .scan(&config, 1, &AtomicU64::new(0))
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
            index
                .scan(&config, 1, &AtomicU64::new(0))
                .unwrap()
                .0
                .complete
        );
        std::fs::remove_file(valid.join("Previous.exe")).unwrap();
        create_executable(&valid.join("Current.exe"));
        std::fs::write(broken.join("AppxManifest.xml"), "invalid manifest").unwrap();
        config.roots.pop();
        let (report, entries) = index.scan(&config, 2, &AtomicU64::new(0)).unwrap();
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
        index
            .scan(&config, 1, &AtomicU64::new(0))
            .expect("first scan should complete");
        std::fs::remove_file(applications.join("First.exe"))
            .expect("test executable should remove");
        let (report, entries) = index
            .scan(&config, 2, &AtomicU64::new(2))
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
        let (report, initial) = index.scan(&config, 1, &AtomicU64::new(0)).unwrap();
        assert!(report.complete);
        assert_eq!(initial.len(), 3);
        std::fs::remove_file(shortcut).unwrap();
        std::fs::remove_file(removed.join("Deleted.exe")).unwrap();
        std::fs::remove_dir(&removed).unwrap();
        let (report, remaining) = index.scan(&config, 2, &AtomicU64::new(0)).unwrap();
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
        let (report, entries) = index
            .scan(&config, 1, &AtomicU64::new(0))
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
        let (report, remaining) = index.scan(&disabled, 2, &AtomicU64::new(0)).unwrap();
        assert!(report.complete);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].entry_id, retained.entry_id);
        disabled.roots.clear();
        let (report, empty) = index.scan(&disabled, 3, &AtomicU64::new(0)).unwrap();
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

        let (_, entries) = index
            .scan(&config, 1, &AtomicU64::new(0))
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
                std::fs::read(root.join("icons").join(entry.icon_key).join("128.png")).unwrap(),
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
        let (report, entries) = index.scan(&config, 1, &AtomicU64::new(0)).unwrap();
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
            index
                .scan(&config, 1, &AtomicU64::new(0))
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
        let (report, entries) = index.scan(&config, 1, &AtomicU64::new(0)).unwrap();
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
            index
                .scan(&config, 2, &AtomicU64::new(0))
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
        let mut ids = std::collections::HashSet::from([direct.entry_id]);
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
            assert!(ids.insert(entry.entry_id));
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
    fn failed_index_transactions_leave_a_failed_scan_state() {
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
                "CREATE TRIGGER reject_application_insert BEFORE INSERT ON app_entries BEGIN SELECT RAISE(ABORT, 'rejected by test'); END;",
            )
            .expect("failure trigger should install");
        let config = ApplicationConfig {
            roots: vec![applications],
            exclusions: platform::standard_roots().expect("standard roots"),
            enabled_builtin_roots: Default::default(),
        };

        assert!(index.scan(&config, 1, &AtomicU64::new(0)).is_err());
        let status = observer
            .query_row("SELECT status FROM scan_state WHERE id = 1", [], |row| {
                row.get::<_, String>(0)
            })
            .expect("scan status should read");
        assert_eq!(status, "failed");
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
