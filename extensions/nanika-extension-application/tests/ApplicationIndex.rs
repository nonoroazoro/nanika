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
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
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
        format_version: 1,
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
        format_version: 1,
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

#[cfg(windows)]
#[test]
fn standard_windows_roots_produce_valid_application_metadata() {
    let root = test_root("standard-roots");
    let database =
        ApplicationDatabase::open(root.join("application.db")).expect("database should open");
    let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
    let config = ApplicationConfig {
        format_version: 1,
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

    let database =
        ApplicationDatabase::open(root.join("application.db")).expect("database should open");
    let mut index = ApplicationIndex::new(database, IconCache::new(root.join("icons")));
    let config = ApplicationConfig {
        format_version: 1,
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
        format_version: 1,
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
    let persistence: IPersistFile = shell_link.cast().expect("persistence interface");
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
