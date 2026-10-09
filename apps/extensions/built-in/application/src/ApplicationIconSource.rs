use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Native extraction identity. Registered applications are not filesystem paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ApplicationIconSource {
    File {
        path: PathBuf,
        index: i32
    },
    WindowsApplication {
        app_user_model_id: String,
        package_full_name: String
    }
}
