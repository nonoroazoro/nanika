use serde::{Deserialize, Serialize};

use crate::ConfigStore;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LauncherPreferences {
    pub format_version: u32,
    pub launcher_shortcut: String,
    pub theme: ThemePreference,
    pub hide_on_blur: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl LauncherPreferences {
    pub fn load(store: &ConfigStore, default_shortcut: &str) -> Result<Self, String> {
        let path = store.config_file();
        let config = match std::fs::metadata(&path) {
            Ok(_) => store
                .load::<Self>(&path)
                .map_err(|error| error.to_string())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self {
                format_version: 1,
                launcher_shortcut: default_shortcut.to_owned(),
                theme: ThemePreference::System,
                hide_on_blur: true,
            },
            Err(error) => return Err(error.to_string()),
        };
        config.validate()?;
        Ok(config)
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
        store
            .update::<Self>(
                store.config_file(),
                [
                    ("formatVersion".to_owned(), serde_json::json!(1)),
                    (
                        "launcherShortcut".to_owned(),
                        serde_json::json!(self.launcher_shortcut),
                    ),
                    ("theme".to_owned(), serde_json::json!(self.theme)),
                    (
                        "hideOnBlur".to_owned(),
                        serde_json::json!(self.hide_on_blur),
                    ),
                ],
                Self::validate,
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
