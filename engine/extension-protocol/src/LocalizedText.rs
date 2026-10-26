use crate::{MessageArgument, MessageReference};
use serde::{Deserialize, Serialize};

/// Literal domain content or a reference resolved within its trusted package scope.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LocalizedText {
    Literal(String),
    Message(MessageReference),
    Translated(crate::TranslatedText)
}

impl LocalizedText {
    /// Validate every supplied translation with the destination's existing literal rules.
    pub fn validate_literals(
        &self,
        mut validate: impl FnMut(&str) -> Result<(), String>
    ) -> Result<(), String> {
        match self {
            Self::Literal(value) => validate(value),
            Self::Message(reference) => reference.validate(),
            Self::Translated(text) => {
                text.validate()?;
                for value in text.values() {
                    validate(value)?;
                }
                Ok(())
            }
        }
    }
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
