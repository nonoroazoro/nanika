use crate::{MAX_LOCALE_RESOURCES, SupportedLocale};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Relative package paths. Keys are the complete declaration of available locales.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizationResources {
    pub default_locale: SupportedLocale,
    pub resources: BTreeMap<SupportedLocale, Vec<String>>
}

impl LocalizationResources {
    pub fn validate(&self) -> Result<(), String> {
        if !self.resources.contains_key(&self.default_locale) {
            return Err("Default locale must be declared.".into());
        }
        for (locale, paths) in &self.resources {
            if paths.is_empty() || paths.len() > MAX_LOCALE_RESOURCES {
                return Err(format!("{}: invalid resource count", locale.as_str()));
            }
            let mut seen = std::collections::BTreeSet::new();
            for path in paths {
                if path.len() > 512
                    || !path.ends_with(".ftl")
                    || path.contains(['\\', ':'])
                    || path.chars().any(char::is_control)
                    || path.split('/').any(|part| matches!(part, "" | "." | ".."))
                    || !seen.insert(path)
                {
                    return Err(format!(
                        "{}: invalid or duplicate relative FTL path",
                        locale.as_str()
                    ));
                }
            }
        }
        Ok(())
    }
}
