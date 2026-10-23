use serde::{Deserialize, Serialize};

/// Stable presentation identity, independent of its translated title.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresentationGroup {
    pub id: String,
    pub title: crate::LocalizedText
}

impl PresentationGroup {
    /// Reject conflicting labels within one admitted package or view publication.
    pub fn validate_definitions<'a>(
        groups: impl IntoIterator<Item = &'a Self>
    ) -> Result<(), String> {
        let mut definitions = std::collections::BTreeMap::new();
        for group in groups {
            group.validate()?;
            if let Some(previous) = definitions.insert(&group.id, &group.title)
                && previous != &group.title
            {
                return Err("conflicting presentation group titles for the same id".into());
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 128
            || !self
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err("presentation group id is invalid".into());
        }
        if let crate::LocalizedText::Message(reference) = &self.title {
            reference.validate()?;
        }
        Ok(())
    }
}
