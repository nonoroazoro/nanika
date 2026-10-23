use serde::{Deserialize, Serialize};

use crate::{LanguagePreference, SupportedLocale};

/// Presentation state. Revisions are comparable only within the owning application session.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizationSnapshot {
    pub preference: LanguagePreference,
    pub locale: SupportedLocale,
    pub revision: u64
}

impl LocalizationSnapshot {
    pub fn new(preference: LanguagePreference, languages: Option<&[String]>) -> Self {
        Self {
            preference,
            locale: preference.resolve(languages, None),
            revision: 1
        }
    }

    /// Prepare before persistence; the caller publishes only after its transaction succeeds.
    pub fn updated(
        self,
        preference: LanguagePreference,
        languages: Option<&[String]>
    ) -> Result<Self, String> {
        let locale = preference.resolve(languages, Some(self.locale));
        if preference == self.preference && locale == self.locale {
            return Ok(self);
        }
        // Revisions cross the JSON boundary as JavaScript safe integers.
        if self.revision >= nanika_protocol::MAX_MESSAGE_INTEGER as u64 {
            return Err("Localization revision limit reached.".into());
        }
        Ok(Self {
            preference,
            locale,
            revision: self.revision + 1
        })
    }
}

impl Default for LocalizationSnapshot {
    fn default() -> Self {
        Self::new(LanguagePreference::System, None)
    }
}
