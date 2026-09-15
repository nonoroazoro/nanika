#![cfg(windows)]
#![cfg_attr(windows, allow(unsafe_code))]

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
#[cfg(windows)]
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IPersistFile,
};
#[cfg(windows)]
use windows::Win32::UI::Shell::{IShellLinkDataList, IShellLinkW, SLDF_RUNAS_USER, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{SHOW_WINDOW_CMD, SW_SHOWMAXIMIZED, SW_SHOWNORMAL};
#[cfg(windows)]
use windows::core::{Interface, PCWSTR};

use crate::platform;
use crate::{ApplicationConfig, ApplicationDatabase, ApplicationIndex, DiscoveryState, IconCache};

#[cfg(windows)]
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

#[cfg(windows)]
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
    };
    index
        .scan(&config, 1, &AtomicU64::new(0))
        .expect("first scan should complete");
    std::fs::remove_file(applications.join("First.exe")).expect("test executable should remove");
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

#[cfg(windows)]
#[test]
fn standard_windows_roots_produce_valid_application_metadata() {
    let root = test_root("standard-roots");
    let database =
        ApplicationDatabase::open(root.join("application.db")).expect("database should open");
    let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
    let config = ApplicationConfig {
        roots: Vec::new(),
        exclusions: Vec::new(),
    };
    let (report, entries) = index
        .scan(&config, 1, &AtomicU64::new(0))
        .expect("standard application scan should complete");
    assert!(report.complete);
    assert!(!entries.is_empty());
    assert!(entries.iter().all(|entry| {
        !entry.entry_id.is_empty()
            && !entry.display_name.is_empty()
            && PathBuf::from(&entry.target_path).is_file()
    }));
    drop(index);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[cfg(windows)]
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
    let direct_entry = platform::read_entry(&mut discovery_state, &executable, 1, 1)
        .expect("direct executable should parse")
        .expect("direct executable should contribute an entry");
    let shortcut_entry = platform::read_entry(&mut discovery_state, &shortcut, 1, 1)
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
        );
        let mut entry = platform::read_entry(&mut state, &shortcut, 1, 0)
            .unwrap()
            .unwrap();
        cache.prepare(&mut entry).unwrap();
        images
            .push(std::fs::read(root.join("icons").join(entry.icon_key).join("128.png")).unwrap());
    }
    assert_ne!(
        images[0], images[1],
        "different shortcut icons must not become the DLL's generic icon"
    );
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
    let entries = database.load_active_entries().unwrap();
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
fn shortcuts_with_distinct_activation_settings_are_not_merged() {
    let root = test_root("shortcut-activation");
    let executable = root.join("Sample.exe");
    create_executable(&executable);
    let mut state = DiscoveryState::new();
    let direct = platform::read_entry(&mut state, &executable, 1, 0)
        .unwrap()
        .unwrap();
    let mut identities = std::collections::HashSet::from([direct.entry_id]);
    for (name, elevated, show_command) in [
        ("Elevated", true, SW_SHOWNORMAL),
        ("Maximized", false, SW_SHOWMAXIMIZED),
    ] {
        let shortcut = root.join(format!("{name}.lnk"));
        create_shell_link_configured(&shortcut, &executable, None, elevated, show_command);
        let entry = platform::read_entry(&mut state, &shortcut, 1, 0)
            .unwrap()
            .unwrap();
        assert!(
            identities.insert(entry.entry_id),
            "activation settings must retain separate candidates"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn invalid_windows_executables_are_rejected() {
    let root = test_root("invalid-executable");
    let executable = root.join("Invalid.exe");
    std::fs::write(&executable, []).expect("invalid executable should exist");

    assert!(
        platform::read_entry(&mut DiscoveryState::new(), &executable, 1, 0)
            .expect("invalid executable should not produce an I/O error")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(windows)]
#[test]
fn executable_validation_cache_rechecks_changed_files() {
    let root = test_root("changed-executable");
    let executable = root.join("Changed.exe");
    create_executable(&executable);
    let mut discovery_state = DiscoveryState::new();
    assert!(
        platform::read_entry(&mut discovery_state, &executable, 1, 0)
            .expect("valid executable should parse")
            .is_some()
    );
    std::fs::remove_file(&executable).expect("valid executable should remove");
    std::fs::write(&executable, []).expect("invalid executable should replace it");
    discovery_state.begin_scan();

    assert!(
        platform::read_entry(&mut discovery_state, &executable, 2, 0)
            .expect("changed executable should parse")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(windows)]
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

#[cfg(windows)]
fn create_executable(target: &std::path::Path) {
    let source = std::env::current_exe().expect("test executable path");
    std::fs::hard_link(&source, target)
        .or_else(|_| std::fs::copy(&source, target).map(|_| ()))
        .expect("test executable should exist");
}

#[cfg(windows)]
fn create_shell_link(path: &std::path::Path, target: &std::path::Path) {
    create_shell_link_configured(path, target, None, false, SW_SHOWNORMAL);
}

fn create_shell_link_configured(
    path: &std::path::Path,
    target: &std::path::Path,
    icon: Option<(&std::path::Path, i32)>,
    elevated: bool,
    show_command: SHOW_WINDOW_CMD,
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
