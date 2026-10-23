use super::{_persist_preferences, parse};
use nanika_config::{LauncherPreferences, ThemePreference};
use nanika_host::{OperationError, OperationErrorCode as Code};
use std::cell::RefCell;

#[test]
fn invalid_or_conflicting_shortcuts_do_not_persist_or_release_the_current_shortcut() {
    let current = _preferences("Ctrl+Space");
    for shortcut in ["KeyA", "invalid shortcut", "Ctrl+KeyK"] {
        let updated = _preferences(shortcut);
        let failure = _persist_preferences(
            &updated,
            &current,
            |_| panic!("Rejected shortcuts cannot write preferences"),
            |_| {
                Err(OperationError::with_detail(
                    Code::ShortcutRegistrationFailed,
                    "conflict"
                ))
            },
            |_| panic!("Rejected shortcuts cannot release existing registration")
        )
        .unwrap_err();
        assert!(failure.rollback.is_none());
        assert!(failure.cleanup.is_none());
        if shortcut == "Ctrl+KeyK" {
            assert_eq!(
                failure.primary,
                OperationError::with_detail(Code::ShortcutRegistrationFailed, "conflict")
            );
        }
    }
}

#[test]
fn failed_save_releases_only_the_new_shortcut_and_retains_cleanup_failure() {
    let current = _preferences("Ctrl+Space");
    let updated = _preferences("Ctrl+KeyK");
    let calls = RefCell::new(Vec::new());
    let failure = _persist_preferences(
        &updated,
        &current,
        |_| {
            calls.borrow_mut().push("save");
            Err("disk full".into())
        },
        |_| {
            calls.borrow_mut().push("register");
            Ok(())
        },
        |shortcut| {
            assert_eq!(shortcut, parse(&updated.launcher_shortcut).unwrap());
            calls.borrow_mut().push("cleanup");
            Err("release denied".into())
        }
    )
    .unwrap_err();
    assert_eq!(*calls.borrow(), ["register", "save", "cleanup"]);
    assert_eq!(
        failure.primary,
        OperationError::with_detail(Code::PreferencesSaveFailed, "disk full")
    );
    assert_eq!(
        failure.cleanup.as_deref(),
        Some(&OperationError::with_detail(
            Code::ShortcutCleanupFailed,
            "release denied"
        ))
    );
    assert!(failure.rollback.is_none());
}

#[test]
fn release_failure_keeps_all_recovery_causes_and_runs_both_recovery_steps() {
    let current = _preferences("Ctrl+Space");
    let updated = _preferences("Ctrl+KeyK");
    for rollback_fails in [false, true] {
        for cleanup_fails in [false, true] {
            let calls = RefCell::new(Vec::new());
            let failure = _persist_preferences(
                &updated,
                &current,
                |value| {
                    if value.launcher_shortcut == updated.launcher_shortcut {
                        calls.borrow_mut().push("save");
                        Ok(())
                    } else {
                        calls.borrow_mut().push("rollback");
                        if rollback_fails {
                            Err("restore denied".into())
                        } else {
                            Ok(())
                        }
                    }
                },
                |_| {
                    calls.borrow_mut().push("register");
                    Ok(())
                },
                |shortcut| {
                    if shortcut == parse(&current.launcher_shortcut).unwrap() {
                        calls.borrow_mut().push("release old");
                        Err("old shortcut remains active".into())
                    } else {
                        calls.borrow_mut().push("cleanup");
                        if cleanup_fails {
                            Err("new shortcut remains active".into())
                        } else {
                            Ok(())
                        }
                    }
                }
            )
            .unwrap_err();
            assert_eq!(
                *calls.borrow(),
                ["register", "save", "release old", "rollback", "cleanup"]
            );
            assert_eq!(
                failure.primary,
                OperationError::with_detail(
                    Code::ShortcutReleaseFailed,
                    "old shortcut remains active"
                )
            );
            assert_eq!(failure.rollback.is_some(), rollback_fails);
            assert_eq!(failure.cleanup.is_some(), cleanup_fails);
            assert!(failure.confirmation.is_none());
            let wire = serde_json::to_value(&failure).unwrap();
            assert_eq!(wire["kind"], "HostSettingsFailure");
            assert_eq!(wire["primary"]["code"], "shortcutReleaseFailed");
            assert!(!format!("{failure:?}").contains("remains active"));
            let unconfirmed = crate::HostSettingsFailure::confirmation_failed(
                Some(&failure),
                "read denied".into()
            );
            assert_eq!(unconfirmed.primary, failure.primary);
            assert_eq!(unconfirmed.rollback, failure.rollback);
            assert_eq!(unconfirmed.cleanup, failure.cleanup);
            assert_eq!(
                unconfirmed.confirmation.as_deref(),
                Some(&OperationError::with_detail(
                    Code::PreferencesConfirmationFailed,
                    "read denied"
                ))
            );
            assert!(failure.confirmation.is_none());
        }
    }
}

#[test]
fn preference_only_changes_do_not_touch_shortcut_registration() {
    let current = _preferences("Ctrl+Space");
    let mut updated = current.clone();
    updated.language = nanika_localization::LanguagePreference::SimplifiedChinese;
    let mut saved = false;
    _persist_preferences(
        &updated,
        &current,
        |value| {
            assert_eq!(value.language, updated.language);
            saved = true;
            Ok(())
        },
        |_| panic!("Language changes do not register shortcuts"),
        |_| panic!("Language changes do not release shortcuts")
    )
    .unwrap();
    assert!(saved);
}

fn _preferences(shortcut: &str) -> LauncherPreferences {
    LauncherPreferences {
        format_version: 1,
        launcher_shortcut: shortcut.into(),
        theme: ThemePreference::System,
        language: nanika_localization::LanguagePreference::English,
        hide_on_blur: true
    }
}
