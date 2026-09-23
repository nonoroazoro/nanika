use std::io::Read;
use std::path::{Path, PathBuf};

use crate::{ApplicationArguments, ApplicationError};

const MAX_SHIM_BYTES: u64 = 64 * 1024;

/// Resolve identity only; native activation continues to use the original wrapper.
pub(super) fn identity(
    executable: &Path,
    arguments: Option<String>,
) -> Result<Option<(PathBuf, ApplicationArguments)>, ApplicationError> {
    let original = || {
        Some((
            executable.to_path_buf(),
            ApplicationArguments::from_windows_raw(arguments.clone()),
        ))
    };
    if !executable
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name.eq_ignore_ascii_case("shims"))
    {
        return Ok(original());
    }
    let file = match std::fs::File::open(executable.with_extension("shim")) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(original()),
        Err(error) => return Err(error.into()),
    };
    let mut content = String::new();
    file.take(MAX_SHIM_BYTES + 1).read_to_string(&mut content)?;
    if content.len() as u64 > MAX_SHIM_BYTES {
        return Err(_invalid("metadata exceeds 64 KiB"));
    }
    let mut target = None;
    let mut prefix = None;
    for line in content
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
    {
        if line.is_empty() || line.starts_with(['#', ';']) || line.starts_with("//") {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(_invalid("expected a key/value field"));
        };
        let value = value.trim();
        // Other fields affect activation semantics. Preserve those wrappers as distinct
        // applications rather than equating them with a plain executable.
        match key.trim().to_ascii_lowercase().as_str() {
            "path" if target.is_none() => target = Some(value),
            "args" if prefix.is_none() => prefix = Some(value),
            "path" | "args" => return Err(_invalid("duplicate identity field")),
            _ => return Ok(original()),
        }
    }
    let target = target.ok_or_else(|| _invalid("missing target path"))?;
    let target = target
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(target);
    let target = PathBuf::from(target);
    let prefix = prefix.unwrap_or_default();
    // Variable expansion differs between wrapper implementations. Keep such launch
    // variants separate unless their semantics can be established unambiguously.
    if !target.is_absolute() || target.to_string_lossy().contains('%') || prefix.contains('%') {
        return Ok(original());
    }
    let target = match target.canonicalize() {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let arguments = [prefix, arguments.as_deref().unwrap_or_default()]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Some((
        target,
        ApplicationArguments::from_windows_raw(Some(arguments)),
    )))
}

fn _invalid(reason: &str) -> ApplicationError {
    ApplicationError::Configuration(format!("invalid executable shim: {reason}"))
}

#[cfg(test)]
#[path = "../../tests/platform/scoop_shim.rs"]
mod tests;
