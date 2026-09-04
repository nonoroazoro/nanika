use serde::{Deserialize, Serialize};

/// Arguments passed to a launched program without implicit shell parsing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchArguments {
    Structured { values: Vec<String> },
    WindowsRaw { value: String },
}

impl Default for LaunchArguments {
    fn default() -> Self {
        Self::Structured { values: Vec::new() }
    }
}
