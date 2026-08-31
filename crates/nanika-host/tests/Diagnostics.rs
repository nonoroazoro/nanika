use std::ffi::OsStr;

use crate::diagnostics::{enforce_log_budget, maximum_level};

#[test]
fn verbose_diagnostics_are_opt_in() {
    assert_eq!(maximum_level(None), tracing::Level::INFO);
    assert_eq!(
        maximum_level(Some(OsStr::new("verbose"))),
        tracing::Level::DEBUG
    );
    assert_eq!(
        maximum_level(Some(OsStr::new("debug"))),
        tracing::Level::INFO
    );
}

#[test]
fn log_budget_removes_oldest_owned_files() {
    let root = std::env::temp_dir().join(format!("nanika-log-budget-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("log directory should exist");
    let oldest = root.join("nanika.2026-08-22.log");
    let newest = root.join("nanika.2026-08-23.log");
    let unrelated = root.join("other.log");
    std::fs::write(&oldest, vec![0_u8; 20]).expect("old log should exist");
    std::fs::write(&newest, vec![0_u8; 20]).expect("new log should exist");
    std::fs::write(&unrelated, vec![0_u8; 40]).expect("unrelated log should exist");

    let retained = enforce_log_budget(&root, 32).expect("budget should apply");

    assert_eq!(retained, 20);
    assert!(!oldest.exists());
    assert!(newest.is_file());
    assert!(unrelated.is_file());
    std::fs::remove_dir_all(root).expect("test directory should be removable");
}
