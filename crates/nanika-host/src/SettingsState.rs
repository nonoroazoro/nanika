use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::Receiver;

use nanika_platform::{PlatformError, StartupStatus};
use nanika_protocol::SettingsContribution;

use crate::HostConfig;

pub(crate) struct SettingsState {
    pub(crate) visible: bool,
    pub(crate) runtime_ready: bool,
    pub(crate) hotkey: String,
    pub(crate) reduced_motion: bool,
    pub(crate) startup_status: Option<StartupStatus>,
    pub(crate) startup_response: Option<Receiver<Result<StartupStatus, PlatformError>>>,
    pub(crate) saving_host: bool,
    pub(crate) drafts: BTreeMap<String, SettingsContribution>,
    pub(crate) dirty: BTreeSet<String>,
    pub(crate) pending_extensions: BTreeMap<String, String>,
    pub(crate) error: Option<String>,
}

impl SettingsState {
    pub(crate) fn new(config: &HostConfig) -> Self {
        Self {
            visible: false,
            runtime_ready: false,
            hotkey: config.hotkey.clone(),
            reduced_motion: config.reduced_motion,
            startup_status: None,
            startup_response: None,
            saving_host: false,
            drafts: BTreeMap::new(),
            dirty: BTreeSet::new(),
            pending_extensions: BTreeMap::new(),
            error: None,
        }
    }

    pub(crate) fn set_contribution(
        &mut self,
        extension_id: String,
        contribution: SettingsContribution,
    ) {
        if !self.dirty.contains(&extension_id) {
            self.drafts.insert(extension_id, contribution);
        }
    }

    pub(crate) fn begin_extension_update(
        &mut self,
        extension_id: String,
        request_id: String,
    ) -> bool {
        if self.pending_extensions.contains_key(&extension_id) {
            return false;
        }
        self.pending_extensions.insert(extension_id, request_id);
        true
    }

    pub(crate) fn finish_extension_update(&mut self, extension_id: &str, request_id: &str) -> bool {
        if self
            .pending_extensions
            .get(extension_id)
            .is_none_or(|pending| pending != request_id)
        {
            return false;
        }
        self.pending_extensions.remove(extension_id);
        true
    }
}
