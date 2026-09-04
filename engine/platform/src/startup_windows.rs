use std::path::Path;

use windows_registry::CURRENT_USER;

use crate::{PlatformError, StartupStatus};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Nanika";
const FILE_NOT_FOUND_HRESULT: i32 = 0x8007_0002_u32 as i32;

pub(crate) fn status(executable: &Path) -> Result<StartupStatus, PlatformError> {
    let expected = startup_command(executable)?;
    let key = match CURRENT_USER.options().read().open(RUN_KEY) {
        Ok(key) => key,
        Err(error) if error.code().0 == FILE_NOT_FOUND_HRESULT => {
            return Ok(StartupStatus::Disabled);
        }
        Err(error) => return Err(registry_error(error)),
    };
    let value = key
        .values()
        .map_err(registry_error)?
        .find(|(name, _)| name == VALUE_NAME);
    let Some((_, value)) = value else {
        return Ok(StartupStatus::Disabled);
    };
    let Ok(value) = String::try_from(value) else {
        return Ok(StartupStatus::NeedsRepair);
    };
    Ok(classify_registration(Some(&value), &expected))
}

pub(crate) fn set_enabled(
    executable: &Path,
    enabled: bool,
) -> Result<StartupStatus, PlatformError> {
    if !enabled && status(executable)? == StartupStatus::Disabled {
        return Ok(StartupStatus::Disabled);
    }
    let key = CURRENT_USER
        .options()
        .read()
        .write()
        .create()
        .open(RUN_KEY)
        .map_err(registry_error)?;
    let previous = key
        .values()
        .map_err(registry_error)?
        .find(|(name, _)| name == VALUE_NAME)
        .map(|(_, value)| value);
    if enabled {
        key.set_string(VALUE_NAME, startup_command(executable)?)
            .map_err(registry_error)?;
    } else if key
        .values()
        .map_err(registry_error)?
        .any(|(name, _)| name == VALUE_NAME)
    {
        key.remove_value(VALUE_NAME).map_err(registry_error)?;
    }
    let result = status(executable);
    let expected = if enabled {
        StartupStatus::Enabled
    } else {
        StartupStatus::Disabled
    };
    if result.as_ref().is_ok_and(|status| *status == expected) {
        return result;
    }
    match previous {
        Some(value) => key.set_value(VALUE_NAME, &value).map_err(registry_error)?,
        None if key
            .values()
            .map_err(registry_error)?
            .any(|(name, _)| name == VALUE_NAME) =>
        {
            key.remove_value(VALUE_NAME).map_err(registry_error)?;
        }
        None => {}
    }
    result.and_then(|status| {
        Err(PlatformError::Message(format!(
            "Windows startup registration verification returned {status:?}"
        )))
    })
}

fn classify_registration(value: Option<&str>, expected: &str) -> StartupStatus {
    match value {
        None => StartupStatus::Disabled,
        Some(value) if value == expected => StartupStatus::Enabled,
        Some(_) => StartupStatus::NeedsRepair,
    }
}

fn startup_command(executable: &Path) -> Result<String, PlatformError> {
    if !executable.is_absolute() {
        return Err(PlatformError::Message(
            "startup executable must be absolute".to_owned(),
        ));
    }
    let executable = executable.to_string_lossy();
    if executable.contains('"') {
        return Err(PlatformError::Message(
            "startup executable contains an invalid quote".to_owned(),
        ));
    }
    Ok(format!("\"{executable}\" --background"))
}

fn registry_error(error: impl std::fmt::Display) -> PlatformError {
    PlatformError::Message(format!("Windows startup registry error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{classify_registration, startup_command};
    use crate::StartupStatus;

    #[test]
    fn command_quotes_the_executable_and_starts_hidden() {
        let command = startup_command(std::path::Path::new(r"C:\Program Files\Nanika\nanika.exe"))
            .expect("command should be valid");
        assert_eq!(
            command,
            r#""C:\Program Files\Nanika\nanika.exe" --background"#
        );
    }

    #[test]
    fn stale_registration_requires_repair() {
        assert_eq!(
            classify_registration(Some(r#""C:\Old\nanika.exe" --background"#), "expected"),
            StartupStatus::NeedsRepair
        );
        assert_eq!(
            classify_registration(None, "expected"),
            StartupStatus::Disabled
        );
    }
}
