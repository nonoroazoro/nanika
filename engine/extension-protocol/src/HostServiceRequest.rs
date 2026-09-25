use serde::{Deserialize, Serialize};

use crate::{ClipboardContent, LaunchDescriptor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "service", rename_all = "camelCase")]
pub enum HostServiceRequest {
    RevealPath { path: String },
    Launch { descriptor: LaunchDescriptor },
    WriteClipboard { content: ClipboardContent },
}
