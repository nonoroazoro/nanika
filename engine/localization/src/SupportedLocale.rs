use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SupportedLocale {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-Hans")]
    SimplifiedChinese
}

impl SupportedLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-Hans"
        }
    }

    /// Match supported UI languages in OS preference order, without matching Hant as Hans.
    pub fn negotiate(languages: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        languages
            .into_iter()
            .find_map(|value| Self::_match(value.as_ref()))
            .unwrap_or(Self::English)
    }

    fn _match(value: &str) -> Option<Self> {
        let language = value.parse::<unic_langid::LanguageIdentifier>().ok()?;
        match language.language.as_str() {
            "en" => Some(Self::English),
            "zh" => {
                let simplified = match language.script {
                    Some(script) => script.as_str() == "Hans",
                    None => language
                        .region
                        .is_none_or(|region| matches!(region.as_str(), "CN" | "SG"))
                };
                simplified.then_some(Self::SimplifiedChinese)
            }
            _ => None
        }
    }
}
