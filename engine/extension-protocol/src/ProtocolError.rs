use crate::LocalizedText;
use serde::{Deserialize, Serialize};

/// A peer-reported failure. The receiver binds message references to the sender's resources.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProtocolError {
    pub code: String,
    pub message: LocalizedText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub detail: Option<String>
}

impl ProtocolError {
    pub fn new(code: impl Into<String>, message: impl Into<LocalizedText>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.code.is_empty()
            || self.code.len() > 128
            || !self
                .code
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err("Invalid error code.".into());
        }
        match &self.message {
            LocalizedText::Literal(message)
                if message.trim().is_empty()
                    || message.chars().any(|character| {
                        character.is_control() && !matches!(character, '\n' | '\r' | '\t')
                    })
                    || message.chars().count() > crate::MAX_ERROR_MESSAGE_CHARS =>
            {
                return Err("Invalid error message.".into());
            }
            LocalizedText::Message(reference) => reference.validate()?,
            _ => {}
        }
        if self.detail.as_ref().is_some_and(|detail| {
            detail.len() > crate::MAX_ERROR_DETAIL_BYTES || detail.contains('\0')
        }) {
            return Err("Invalid error diagnostic detail.".into());
        }
        Ok(())
    }
}

impl std::fmt::Debug for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reference = match &self.message {
            LocalizedText::Message(reference) => Some(reference.message.as_str()),
            LocalizedText::Literal(_) => None
        };
        formatter
            .debug_struct("ProtocolError")
            .field("code", &self.code)
            .field("message_id", &reference)
            .field("detail", &self.detail.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: ", self.code)?;
        match &self.message {
            LocalizedText::Literal(message) => formatter.write_str(message)?,
            // Diagnostics identify the reference without disclosing its argument values.
            LocalizedText::Message(reference) => write!(formatter, "message {}", reference.message)?
        }
        if let Some(detail) = &self.detail {
            write!(formatter, ": {detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProtocolError {}
