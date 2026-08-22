use std::str::FromStr;

use global_hotkey::hotkey::HotKey;
use nanika_config::ConfigStore;
use serde::{Deserialize, Serialize};

const FORMAT_VERSION: u32 = 1;

/// Host-owned preferences stored in the root Nanika config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostConfig {
    pub(crate) format_version: u32,
    pub(crate) hotkey: String,
    #[serde(default)]
    pub(crate) reduced_motion: bool,
}

impl HostConfig {
    pub(crate) fn load(store: &ConfigStore) -> Result<Self, String> {
        let path = store.config_file();
        if !path.exists() {
            let config = Self::default();
            if !store.is_read_only() {
                store
                    .save(&path, &config)
                    .map_err(|error| error.to_string())?;
            }
            return Ok(config);
        }
        let config = store
            .load::<Self>(&path)
            .map_err(|error| error.to_string())?;
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "unsupported host settings format {}",
                self.format_version
            ));
        }
        HotKey::from_str(&self.hotkey)
            .map(|_| ())
            .map_err(|error| format!("invalid global hotkey: {error}"))
    }
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            hotkey: "Ctrl+Space".to_owned(),
            reduced_motion: false,
        }
    }
}
