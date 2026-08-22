use std::path::PathBuf;

use nanika_protocol::Candidate;

use crate::RUN_ACTION_ID;

/// Persisted application metadata plus transient icon extraction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntry {
    pub entry_id: String,
    pub source_key: String,
    pub display_name: String,
    pub normalized_name: String,
    pub normalized_tokens: String,
    pub launch_kind: String,
    pub target_path: String,
    pub working_directory: Option<String>,
    pub arguments_json: String,
    pub bundle_id: Option<String>,
    pub icon_key: String,
    pub file_identity: String,
    pub last_seen_at: u64,
    pub stale: bool,
    pub(crate) icon_source: Option<PathBuf>,
    pub(crate) icon_index: i32,
    pub(crate) priority: usize,
}

impl ApplicationEntry {
    pub fn candidate(&self) -> Candidate {
        Candidate {
            entry_id: self.entry_id.clone(),
            title: self.display_name.clone(),
            action_id: RUN_ACTION_ID.to_owned(),
            aliases: self
                .normalized_tokens
                .split_whitespace()
                .filter(|alias| *alias != self.normalized_name)
                .map(str::to_owned)
                .collect(),
        }
    }
}
