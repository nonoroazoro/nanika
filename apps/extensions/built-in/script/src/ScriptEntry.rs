use nanika_protocol::{Candidate, CandidateKind, LaunchDescriptor};

use crate::RUN_ACTION_ID;

/// A discovered script. Identity follows its canonical path, not its display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEntry {
    _data: std::sync::Arc<crate::ScriptEntryData>,
}

impl std::ops::Deref for ScriptEntry {
    type Target = crate::ScriptEntryData;
    fn deref(&self) -> &Self::Target {
        &self._data
    }
}

impl std::ops::DerefMut for ScriptEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::sync::Arc::make_mut(&mut self._data)
    }
}

impl ScriptEntry {
    pub fn new(data: crate::ScriptEntryData) -> Self {
        Self {
            _data: std::sync::Arc::new(data),
        }
    }

    pub fn candidate(&self) -> Candidate {
        Candidate {
            kind: CandidateKind::Action,
            entry_id: self.id.clone(),
            title: self.title.clone(),
            subtitle: Some(nanika_protocol::CandidateSubtitle::Description(
                self.path.to_string_lossy().into_owned(),
            )),
            action_id: RUN_ACTION_ID.to_owned(),
            actions: vec![nanika_protocol::Action::primary(
                RUN_ACTION_ID.to_owned(),
                "Run",
            )],
            aliases: Vec::new(),
            icon: None,
        }
    }

    pub fn launch_descriptor(&self) -> Result<LaunchDescriptor, String> {
        crate::platform::launch_descriptor(&self.path)
    }
}
