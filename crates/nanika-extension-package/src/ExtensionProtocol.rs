use serde::{Deserialize, Serialize};

/// Versioned wire protocol used by one extension process.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "protocol", rename_all = "camelCase", deny_unknown_fields)]
pub enum ExtensionProtocol {
    Nanika {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
    },
    Acp {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
    },
}

impl ExtensionProtocol {
    pub fn validate(self) -> Result<(), String> {
        match self {
            Self::Nanika {
                protocol_version: 1,
            }
            | Self::Acp {
                protocol_version: 1,
            } => Ok(()),
            Self::Nanika { protocol_version } => Err(format!(
                "unsupported Nanika extension protocol version: {protocol_version}"
            )),
            Self::Acp { protocol_version } => Err(format!(
                "unsupported ACP protocol version: {protocol_version}"
            )),
        }
    }
}
