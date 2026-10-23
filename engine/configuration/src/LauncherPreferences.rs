use serde::{Deserialize, Serialize};

use crate::ConfigStore;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LauncherPreferences {
    #[cfg_attr(feature = "typescript", ts(type = "1"))]
    pub format_version: u32,
    pub launcher_shortcut: String,
    pub theme: ThemePreference,
    #[serde(default)]
    pub language: nanika_localization::LanguagePreference,
    pub hide_on_blur: bool
}

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    System,
    Light,
    Dark
}

impl LauncherPreferences {
    pub fn load(store: &ConfigStore, default_shortcut: &str) -> Result<Self, String> {
        let options = store
            .options(store.config_file())
            .map_err(|error| error.to_string())?;
        Ok(Self {
            format_version: 1,
            launcher_shortcut: options.get_validated(
                "launcherShortcut",
                default_shortcut.to_owned(),
                |value: &String| {
                    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)
                    {
                        Err("Invalid launcher shortcut.".into())
                    } else {
                        Ok(())
                    }
                }
            ),
            theme: options.get("theme", ThemePreference::System),
            language: options.get("language", nanika_localization::LanguagePreference::System),
            hide_on_blur: options.get("hideOnBlur", true)
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != 1
            || self.launcher_shortcut.is_empty()
            || self.launcher_shortcut.len() > 128
            || self.launcher_shortcut.chars().any(char::is_control)
        {
            return Err("Invalid launcher preferences.".to_owned());
        }
        Ok(())
    }

    pub fn save(&self, store: &ConfigStore) -> Result<(), String> {
        self.validate()?;
        if !store
            .load::<Self>(store.config_file())
            .is_ok_and(|stored| stored.validate().is_ok())
        {
            return store
                .save(store.config_file(), self)
                .map_err(|error| error.to_string());
        }
        store
            .update::<Self>(
                store.config_file(),
                [
                    ("formatVersion".to_owned(), serde_json::json!(1)),
                    (
                        "launcherShortcut".to_owned(),
                        serde_json::json!(self.launcher_shortcut)
                    ),
                    ("theme".to_owned(), serde_json::json!(self.theme)),
                    ("language".to_owned(), serde_json::json!(self.language)),
                    (
                        "hideOnBlur".to_owned(),
                        serde_json::json!(self.hide_on_blur)
                    )
                ],
                Self::validate
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
