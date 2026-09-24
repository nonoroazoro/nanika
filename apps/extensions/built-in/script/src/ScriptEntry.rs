use std::path::PathBuf;

use nanika_protocol::{Candidate, CandidateKind, ContributionIcon, LaunchDescriptor};

use crate::RUN_ACTION_ID;

/// A discovered script. Identity follows its canonical path, not its display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEntry {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
}

impl ScriptEntry {
    pub fn candidate(&self) -> Candidate {
        Candidate {
            kind: CandidateKind::Action,
            entry_id: self.id.clone(),
            title: self.title.clone(),
            subtitle: Some(self.path.to_string_lossy().into_owned()),
            action_id: RUN_ACTION_ID.to_owned(),
            actions: vec![nanika_protocol::Action::primary(
                RUN_ACTION_ID.to_owned(),
                "Run",
            )],
            aliases: Vec::new(),
            icon: None,
            contribution_icon: Some(ContributionIcon::Script),
        }
    }

    pub fn launch_descriptor(&self) -> Result<LaunchDescriptor, String> {
        crate::platform::launch_descriptor(&self.path)
    }
}
