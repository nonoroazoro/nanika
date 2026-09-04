use std::path::PathBuf;

use nanika_protocol::{Candidate, IconReference, LaunchArguments, LaunchDescriptor};

use crate::{ApplicationArguments, ApplicationError, RUN_ACTION_ID};

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
            subtitle: Some("Application".to_owned()),
            action_id: RUN_ACTION_ID.to_owned(),
            aliases: self
                .normalized_tokens
                .lines()
                .filter(|alias| *alias != self.normalized_name)
                .map(str::to_owned)
                .collect(),
            icon: IconReference::new(&self.icon_key).ok(),
        }
    }

    pub fn launch_descriptor(&self) -> Result<LaunchDescriptor, ApplicationError> {
        if self.launch_kind == "macos-bundle" {
            return Ok(LaunchDescriptor::MacApplication {
                bundle_path: self.target_path.clone(),
            });
        }
        let arguments = match serde_json::from_str::<ApplicationArguments>(&self.arguments_json)? {
            ApplicationArguments::Structured { values } => LaunchArguments::Structured { values },
            ApplicationArguments::WindowsRaw { value } => LaunchArguments::WindowsRaw { value },
        };
        Ok(LaunchDescriptor::Program {
            program: self.target_path.clone(),
            arguments,
            working_directory: self.working_directory.clone(),
        })
    }
}
