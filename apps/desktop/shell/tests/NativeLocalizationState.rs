use super::NativeLocalizationState;
use nanika_localization::{LanguagePreference, LocalizationSnapshot};

#[test]
fn partial_updates_retry_and_cannot_be_overwritten_by_an_older_request() {
    let english = LocalizationSnapshot::new(LanguagePreference::English, None);
    let chinese = english
        .updated(LanguagePreference::SimplifiedChinese, None)
        .unwrap();
    let restored = chinese.updated(LanguagePreference::English, None).unwrap();
    let mut state = NativeLocalizationState::default();
    let mut labels = ["English", "English"];
    state.apply(english, || Ok(())).unwrap();
    assert!(
        state
            .apply(chinese, || {
                labels[0] = "Chinese";
                Err("Native item unavailable".into())
            })
            .is_err()
    );
    state
        .apply(english, || {
            panic!("an old request cannot overwrite a newer partial update")
        })
        .unwrap();
    state
        .apply(restored, || {
            labels = ["English", "English"];
            Ok(())
        })
        .unwrap();
    assert_eq!(labels, ["English", "English"]);
    state
        .apply(restored, || panic!("a completed update does not repaint"))
        .unwrap();
}

#[test]
fn retrying_the_failed_revision_reapplies_all_native_text() {
    let snapshot = LocalizationSnapshot::default();
    let mut state = NativeLocalizationState::default();
    assert!(
        state
            .apply(snapshot, || Err("Native update failed".into()))
            .is_err()
    );
    let mut applied = false;
    state
        .apply(snapshot, || {
            applied = true;
            Ok(())
        })
        .unwrap();
    assert!(applied);
    let explicit = snapshot.updated(LanguagePreference::English, None).unwrap();
    state
        .apply(explicit, || {
            panic!("a preference-only change does not repaint native UI")
        })
        .unwrap();
}
