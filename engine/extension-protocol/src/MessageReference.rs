use crate::{MessageArgument, MessageArguments};
use serde::{Deserialize, Serialize};
#[cfg(feature = "typescript")]
use std::collections::BTreeMap;

/// Resource ownership is supplied by the host, never by the sender.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MessageReference {
    pub message: String,
    #[serde(default, skip_serializing_if = "MessageArguments::is_empty")]
    #[cfg_attr(
        feature = "typescript",
        ts(as = "Option<BTreeMap<String, MessageArgument>>", optional)
    )]
    pub args: MessageArguments
}

impl MessageReference {
    /// Validate shape before consulting the package's message contract.
    pub fn validate(&self) -> Result<(), String> {
        if !is_message_identifier(&self.message) {
            return Err("Invalid message identifier.".into());
        }
        if self.args.len() > crate::MAX_MESSAGE_ARGUMENTS {
            return Err("Too many message arguments.".into());
        }
        for (name, value) in &self.args {
            if name == "platform" || !is_message_identifier(name) {
                return Err("Invalid or reserved message argument name.".into());
            }
            match value {
                MessageArgument::Text(value)
                    if value.len() > crate::MAX_MESSAGE_ARGUMENT_BYTES || value.contains('\0') =>
                {
                    return Err(format!("Invalid text argument: {name}"));
                }
                MessageArgument::Integer(value)
                    if !(-crate::MAX_MESSAGE_INTEGER..=crate::MAX_MESSAGE_INTEGER)
                        .contains(value) =>
                {
                    return Err(format!("Unsafe integer argument: {name}"));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

pub fn is_message_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= crate::MAX_MESSAGE_ID_BYTES
        && bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
