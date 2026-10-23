use crate::SupportedLocale;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguagePreference {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-Hans")]
    SimplifiedChinese
}

impl LanguagePreference {
    /// None means a failed OS read; retain the last effective locale after startup.
    pub fn resolve(
        self,
        languages: Option<&[String]>,
        previous: Option<SupportedLocale>
    ) -> SupportedLocale {
        match self {
            Self::English => SupportedLocale::English,
            Self::SimplifiedChinese => SupportedLocale::SimplifiedChinese,
            Self::System => languages
                .map(SupportedLocale::negotiate)
                .or(previous)
                .unwrap_or(SupportedLocale::English)
        }
    }
}
