use std::path::PathBuf;

use crate::EXTENSION_ID;

pub struct RuntimePaths {
    pub data_root: PathBuf,
    pub cache_root: PathBuf,
}

impl RuntimePaths {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut data_root = None;
        let mut cache_root = None;
        for argument in arguments {
            if let Some(value) = argument.strip_prefix("--data-root=") {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("clipboard extension data root must be absolute".to_owned());
                }
                data_root = Some(path);
            } else if let Some(value) = argument.strip_prefix("--cache-root=") {
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("clipboard cache root must be absolute".to_owned());
                }
                cache_root = Some(path);
            } else {
                return Err(format!(
                    "unsupported clipboard extension argument: {argument}"
                ));
            }
        }
        Ok(Self {
            data_root: data_root.ok_or_else(|| "clipboard data root is missing".to_owned())?,
            cache_root: cache_root.ok_or_else(|| "clipboard cache root is missing".to_owned())?,
        })
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_root
            .join("databases/extensions")
            .join(format!("{EXTENSION_ID}.db"))
    }

    pub fn payload_root(&self) -> PathBuf {
        self.data_root.join("payloads").join(EXTENSION_ID)
    }
}
