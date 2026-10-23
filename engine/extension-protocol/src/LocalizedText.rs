use crate::{MessageArgument, MessageReference};
use serde::{Deserialize, Serialize};

/// Literal domain content or a reference resolved within its trusted package scope.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LocalizedText {
    Literal(String),
    Message(MessageReference)
}

impl From<String> for LocalizedText {
    fn from(value: String) -> Self {
        Self::Literal(value)
    }
}

impl From<&str> for LocalizedText {
    fn from(value: &str) -> Self {
        Self::Literal(value.to_owned())
    }
}

/// Construct a reference without formatting, locale state, or a Fluent dependency.
pub fn message(
    id: impl Into<String>,
    args: impl IntoIterator<Item = (String, MessageArgument)>
) -> LocalizedText {
    LocalizedText::Message(MessageReference {
        message: id.into(),
        args: args.into_iter().collect()
    })
}
