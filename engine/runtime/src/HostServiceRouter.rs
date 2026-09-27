use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use nanika_platform::{ClipboardService, ProcessLauncher, SystemActionService};
use nanika_protocol::{ClipboardContent, HostServiceRequest};
use nanika_storage::is_valid_extension_id;

use crate::{DiagnosticCode, HostDiagnostic, HostServiceHandler};

pub struct HostServiceRouter {
    system: OnceLock<Result<SystemActionService, String>>,
    launcher: Result<ProcessLauncher, String>,
    clipboard: Result<ClipboardService, String>,
    payload_root: Result<PathBuf, String>,
    permissions: RwLock<HashMap<String, HashSet<String>>>,
}

impl HostServiceRouter {
    pub fn spawn(app_data_root: &Path) -> (Self, Vec<String>) {
        let mut errors = Vec::new();
        let payload_root = app_data_root.join("payloads");
        let payload_root = std::fs::create_dir_all(&payload_root)
            .and_then(|()| payload_root.canonicalize())
            .map_err(|error| format!("extension payload service is unavailable: {error}"));
        let launcher = ProcessLauncher::spawn()
            .map_err(|error| format!("process launch service is unavailable: {error}"));
        let clipboard = ClipboardService::spawn()
            .map_err(|error| format!("clipboard service is unavailable: {error}"));
        if let Err(error) = &payload_root {
            errors.push(error.clone());
        }
        if let Err(error) = &launcher {
            errors.push(error.clone());
        }
        if let Err(error) = &clipboard {
            errors.push(error.clone());
        }
        (
            Self {
                system: OnceLock::new(),
                launcher,
                clipboard,
                payload_root,
                permissions: RwLock::new(HashMap::new()),
            },
            errors,
        )
    }

    fn payload_root(&self) -> Result<&Path, String> {
        self.payload_root.as_deref().map_err(Clone::clone)
    }

    fn launcher(&self) -> Result<&ProcessLauncher, String> {
        self.launcher.as_ref().map_err(Clone::clone)
    }

    fn clipboard(&self) -> Result<&ClipboardService, String> {
        self.clipboard.as_ref().map_err(Clone::clone)
    }

    fn extension_payload_root(&self, extension_id: &str) -> Result<PathBuf, String> {
        Ok(self.payload_root()?.join(extension_id))
    }

    pub(crate) fn register_permissions(
        &self,
        extension_id: impl Into<String>,
        permissions: impl IntoIterator<Item = String>,
    ) {
        self.permissions
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .insert(extension_id.into(), permissions.into_iter().collect());
    }

    fn require_permission(&self, extension_id: &str, permission: &str) -> Result<(), String> {
        if self
            .permissions
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(extension_id)
            .is_some_and(|permissions| permissions.contains(permission))
        {
            return Ok(());
        }
        HostDiagnostic::new(
            DiagnosticCode::PermissionDenied,
            "authorize extension host service",
            "An extension requested a host service without permission.",
        )
        .with_safe_context(extension_id)
        .record_warning();
        Err(format!(
            "extension {extension_id} lacks permission {permission}"
        ))
    }
}

impl HostServiceHandler for HostServiceRouter {
    fn prepare(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
        interruption: &mut dyn FnMut() -> crate::ExtensionInterruption,
    ) -> Result<crate::PreparedHostService<'_>, String> {
        if !is_valid_extension_id(extension_id) {
            HostDiagnostic::new(
                DiagnosticCode::PermissionDenied,
                "validate host service caller",
                "An invalid extension identity requested a host service.",
            )
            .with_safe_context("invalid-extension-id")
            .record_warning();
            return Err("host service request has an invalid extension id".to_owned());
        }
        match request {
            HostServiceRequest::SystemAction { action } => {
                self.require_permission(extension_id, action.permission())?;
                let service = self
                    .system
                    .get_or_init(|| {
                        SystemActionService::spawn()
                            .map_err(|error| format!("System service is unavailable: {error}"))
                    })
                    .as_ref()
                    .map_err(Clone::clone)?;
                Ok(crate::PreparedHostService::new(move || {
                    service.submit(action)
                }))
            }
            HostServiceRequest::RevealPath { path } => {
                self.require_permission(extension_id, "files.reveal")?;
                let service = self.launcher()?;
                Ok(crate::PreparedHostService::new(move || {
                    service.reveal(path)
                }))
            }
            HostServiceRequest::Launch { descriptor } => {
                self.require_permission(extension_id, "process.launch")?;
                let service = self.launcher()?;
                Ok(crate::PreparedHostService::new(move || {
                    service.submit(descriptor)
                }))
            }
            HostServiceRequest::WriteClipboard { content } => {
                self.require_permission(extension_id, "clipboard.write")?;
                let payload_root = match &content {
                    ClipboardContent::PngFile { .. } => {
                        Some(self.extension_payload_root(extension_id)?)
                    }
                    ClipboardContent::Text { .. } | ClipboardContent::Files { .. } => None,
                };
                let service = self.clipboard()?;
                let prepared = service.prepare(content, payload_root.as_deref(), &mut || {
                    interruption() != crate::ExtensionInterruption::None
                })?;
                Ok(crate::PreparedHostService::new(move || {
                    service.submit(prepared)
                }))
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/HostServiceRouter.rs"]
mod tests;
