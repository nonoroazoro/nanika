use nanika_protocol::SettingsContribution;

use crate::{HostConfig, SettingsState};

fn contribution(title: &str) -> SettingsContribution {
    SettingsContribution {
        title: title.to_owned(),
        fields: Vec::new(),
    }
}

#[test]
fn extension_update_accepts_only_the_pending_request() {
    let mut state = SettingsState::new(&HostConfig::default());

    assert!(state.begin_extension_update("example".to_owned(), "request-1".to_owned()));
    assert!(!state.begin_extension_update("example".to_owned(), "request-2".to_owned()));
    assert!(!state.finish_extension_update("example", "request-2"));
    assert!(state.finish_extension_update("example", "request-1"));
}

#[test]
fn incoming_contribution_does_not_replace_a_dirty_draft() {
    let mut state = SettingsState::new(&HostConfig::default());
    state.set_contribution("example".to_owned(), contribution("Original"));
    state.dirty.insert("example".to_owned());

    state.set_contribution("example".to_owned(), contribution("Incoming"));

    assert_eq!(state.drafts["example"].title, "Original");
}
