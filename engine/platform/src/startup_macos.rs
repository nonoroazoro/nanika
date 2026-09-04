use std::path::Path;

use objc2_service_management::{SMAppService, SMAppServiceStatus};

use crate::{PlatformError, StartupStatus};

pub(crate) fn status(_executable: &Path) -> Result<StartupStatus, PlatformError> {
    let service = unsafe { SMAppService::mainAppService() };
    Ok(map_status(unsafe { service.status() }))
}

pub(crate) fn set_enabled(
    executable: &Path,
    enabled: bool,
) -> Result<StartupStatus, PlatformError> {
    let service = unsafe { SMAppService::mainAppService() };
    let current = map_status(unsafe { service.status() });
    if enabled {
        match current {
            StartupStatus::Disabled => unsafe { service.registerAndReturnError() }
                .map_err(|error| PlatformError::Message(format!("macOS startup error: {error}")))?,
            StartupStatus::RequiresApproval => {
                unsafe { SMAppService::openSystemSettingsLoginItems() };
                return Ok(StartupStatus::RequiresApproval);
            }
            StartupStatus::NotFound => {
                return Err(PlatformError::Message(
                    "macOS could not find the signed Nanika application bundle".to_owned(),
                ));
            }
            StartupStatus::Enabled | StartupStatus::NeedsRepair => {}
        }
    } else if current != StartupStatus::Disabled {
        unsafe { service.unregisterAndReturnError() }
            .map_err(|error| PlatformError::Message(format!("macOS startup error: {error}")))?;
    }
    status(executable)
}

fn map_status(status: SMAppServiceStatus) -> StartupStatus {
    match status {
        SMAppServiceStatus::NotRegistered => StartupStatus::Disabled,
        SMAppServiceStatus::Enabled => StartupStatus::Enabled,
        SMAppServiceStatus::RequiresApproval => StartupStatus::RequiresApproval,
        SMAppServiceStatus::NotFound => StartupStatus::NotFound,
        _ => StartupStatus::NotFound,
    }
}
