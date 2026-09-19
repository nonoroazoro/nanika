use std::path::PathBuf;

use nanika_protocol::ExtensionConfiguration;

/// Directories supplied through the host-owned settings snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptConfig {
    pub roots: Vec<PathBuf>,
}

impl ScriptConfig {
    pub fn from_configuration(configuration: &ExtensionConfiguration) -> Result<Self, String> {
        let values = configuration
            .values()
            .get("script.roots")
            .and_then(serde_json::Value::as_array)
            .ok_or("script configuration is missing array script.roots")?;
        if values.len() > 256 {
            return Err("script configuration exceeds 256 directories".to_owned());
        }
        let roots = values
            .iter()
            .map(|value| {
                let value = value.as_str().ok_or("script directories must be strings")?;
                let path = PathBuf::from(value);
                if !path.is_absolute() || value.len() > 4096 {
                    return Err(
                        "script directories must be absolute paths of at most 4096 bytes"
                            .to_owned(),
                    );
                }
                Ok(path)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self { roots })
    }
}
