use crate::{ResourceSet, SupportedLocale};
use nanika_protocol::{LocalizedText, ProtocolError};
use serde::Serialize;
use std::collections::BTreeMap;

/// A complete error presentation admitted against the sender's retained resources.
/// Its prepared text remains usable after the failed process or package is replaced.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Serialize, PartialEq, Eq)]
pub struct PreparedError {
    pub error: ProtocolError,
    pub messages: BTreeMap<SupportedLocale, String>
}

impl PreparedError {
    pub fn new(
        error: ProtocolError,
        resources: Option<&ResourceSet>,
        platform: &str
    ) -> Result<Self, String> {
        error.validate()?;
        if !matches!(platform, "macos" | "windows") {
            return Err("Unsupported localization platform.".into());
        }
        let mut messages = BTreeMap::new();
        for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
            let text = match &error.message {
                LocalizedText::Literal(message) => message.clone(),
                LocalizedText::Translated(value) => value.resolve(locale.as_str()).to_owned(),
                LocalizedText::Message(_) => {
                    let resources = resources
                        .ok_or("Error message requires package localization resources.")?;
                    let mut text = String::new();
                    // The receiver admits every supported branch, independently of the current OS/locale.
                    for target in ["macos", "windows"] {
                        let formatted = resources
                            .format(
                                locale,
                                &error.message,
                                target,
                                nanika_protocol::MAX_ERROR_MESSAGE_CHARS,
                                false
                            )
                            .map_err(|error| {
                                format!("error message: {}: {target}: {error}", locale.as_str())
                            })?;
                        if target == platform {
                            text = formatted;
                        }
                    }
                    text
                }
            };
            messages.insert(locale, text);
        }
        Ok(Self { error, messages })
    }
}

impl std::fmt::Display for PreparedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.error)
    }
}

impl std::fmt::Debug for PreparedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl std::error::Error for PreparedError {}
