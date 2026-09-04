use serde::{Deserialize, Serialize};

use crate::{ClipboardContent, LaunchDescriptor};

/// A typed platform service requested by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "service", rename_all = "camelCase")]
pub enum HostServiceRequest {
    Launch { descriptor: LaunchDescriptor },
    WriteClipboard { content: ClipboardContent },
}
