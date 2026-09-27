use serde::{Deserialize, Serialize};

use crate::LaunchArguments;

/// Typed process request executed only by the host launch service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchDescriptor {
    Program {
        program: String,
        #[serde(default)]
        arguments: LaunchArguments,
        working_directory: Option<String>,
    },
    Shell {
        command: String,
        working_directory: Option<String>,
    },
    MacApplication {
        bundle_path: String,
    },
    WindowsPackagedApplication {
        app_user_model_id: String,
    },
    WindowsApplication {
        path: String,
    },
}
