use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{ScriptConfig, ScriptEntry};

/// Build a replacement catalog atomically. A failed scan never publishes partial data.
pub fn discover_scripts(config: &ScriptConfig) -> Result<BTreeMap<String, ScriptEntry>, String> {
    let mut entries = BTreeMap::new();
    for root in &config.roots {
        let metadata = root
            .symlink_metadata()
            .map_err(|error| format!("{}: {error}", root.display()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "script root must be a regular directory: {}",
                root.display()
            ));
        }
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = entry.map_err(|error| error.to_string())?;
            // Do not traverse links outside the selected directory or execute link targets.
            if !entry.file_type().is_file() || !is_script(entry.path()) {
                continue;
            }
            let path = entry
                .path()
                .canonicalize()
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;
            let text = path
                .to_str()
                .ok_or_else(|| format!("script path is not UTF-8: {}", path.display()))?;
            if text.len() > 4096 {
                return Err(format!(
                    "script path exceeds 4096 bytes: {}",
                    path.display()
                ));
            }
            let mut id = String::from("script.");
            for byte in Sha256::digest(text.as_bytes()) {
                let _ = write!(id, "{byte:02x}");
            }
            let title = path
                .file_stem()
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| format!("script filename is empty: {}", path.display()))?
                .to_owned();
            entries.insert(id.clone(), ScriptEntry { id, title, path });
            if entries.len() > 5000 {
                return Err("script directories contain more than 5000 scripts".to_owned());
            }
        }
    }
    Ok(entries)
}

fn is_script(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            ["ps1", "py", "js", "sh"]
                .iter()
                .any(|extension| value.eq_ignore_ascii_case(extension))
        })
}
