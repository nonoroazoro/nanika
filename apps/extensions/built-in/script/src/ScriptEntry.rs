use std::path::PathBuf;

use nanika_protocol::{Candidate, LaunchArguments, LaunchDescriptor};
use serde::{Deserialize, Serialize};

use crate::RUN_ACTION_ID;

/// One configured script and its explicit interpreter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptEntry {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub interpreter: PathBuf,
    pub script: PathBuf,
    #[serde(default)]
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
}

impl ScriptEntry {
    pub fn candidate(&self) -> Candidate {
        Candidate {
            entry_id: format!("script.{}", self.id),
            title: self.title.clone(),
            subtitle: Some("Script".to_owned()),
            action_id: RUN_ACTION_ID.to_owned(),
            aliases: self.aliases.clone(),
            icon: None,
        }
    }

    pub fn launch_descriptor(&self) -> LaunchDescriptor {
        let values = std::iter::once(self.script.to_string_lossy().into_owned())
            .chain(self.arguments.iter().cloned())
            .collect();
        LaunchDescriptor::Program {
            program: self.interpreter.to_string_lossy().into_owned(),
            arguments: LaunchArguments::Structured { values },
            working_directory: self
                .working_directory
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        }
    }
}
