use std::path::PathBuf;

/// Host-supplied machine and configuration roots.
pub struct RuntimePaths {
    pub data_root: PathBuf,
    pub config_root: PathBuf,
}

impl RuntimePaths {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut data_root = None;
        let mut config_root = None;
        for argument in arguments {
            if let Some(value) = argument.strip_prefix("--data-root=") {
                data_root = Some(absolute(value, "data root")?);
            } else if let Some(value) = argument.strip_prefix("--config-root=") {
                config_root = Some(absolute(value, "config root")?);
            } else if argument.starts_with("--cache-root=") {
            } else {
                return Err(format!("unsupported script extension argument: {argument}"));
            }
        }
        Ok(Self {
            data_root: data_root.ok_or_else(|| "script data root is missing".to_owned())?,
            config_root: config_root.ok_or_else(|| "script config root is missing".to_owned())?,
        })
    }
}

fn absolute(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    path.is_absolute()
        .then_some(path)
        .ok_or_else(|| format!("script extension {label} must be absolute"))
}
