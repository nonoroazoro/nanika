use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use nanika_platform::{ClipboardService, ProcessLauncher};
use nanika_protocol::{ClipboardContent, HostServiceRequest, HostServiceResponse};
use nanika_storage::is_valid_extension_id;

use crate::{DiagnosticCode, HostDiagnostic, HostServiceHandler};

/// Routes typed extension requests to host-owned platform services.
pub struct HostServiceRouter {
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
    fn submit(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
        deadline: Instant,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
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
            HostServiceRequest::Launch { descriptor } => {
                self.require_permission(extension_id, "process.launch")?;
                self.launcher()?.submit(descriptor, deadline)
            }
            HostServiceRequest::WriteClipboard { content } => {
                self.require_permission(extension_id, "clipboard.write")?;
                let payload_root = match &content {
                    ClipboardContent::PngFile { .. } => {
                        Some(self.extension_payload_root(extension_id)?)
                    }
                    ClipboardContent::Text { .. } | ClipboardContent::Files { .. } => None,
                };
                self.clipboard()?.submit(content, payload_root, deadline)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nanika_protocol::{
        ClipboardContent, HostServiceRequest, LaunchArguments, LaunchDescriptor,
    };

    use super::{HostServiceHandler, HostServiceRouter, ProcessLauncher};

    #[test]
    fn clipboard_and_payload_unavailability_do_not_disable_process_launch() {
        let router = HostServiceRouter {
            launcher: Ok(ProcessLauncher::spawn().expect("launcher")),
            clipboard: Err("clipboard unavailable".to_owned()),
            payload_root: Err("payload unavailable".to_owned()),
            permissions: Default::default(),
        };
        router.register_permissions("com.nanika.test", ["process.launch".to_owned()]);
        let result = router
            .submit(
                "com.nanika.test",
                HostServiceRequest::Launch {
                    descriptor: LaunchDescriptor::Program {
                        program: String::new(),
                        arguments: LaunchArguments::default(),
                        working_directory: None,
                    },
                },
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )
            .expect("launch service should remain available")
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("launch service response");
        assert!(result.is_err());
    }

    #[test]
    fn payload_roots_reject_invalid_extension_ids() {
        let router = HostServiceRouter {
            launcher: Err("launcher unavailable".to_owned()),
            clipboard: Err("clipboard unavailable".to_owned()),
            payload_root: Ok(std::env::temp_dir()),
            permissions: Default::default(),
        };
        let result = router.submit(
            "../escape",
            HostServiceRequest::WriteClipboard {
                content: ClipboardContent::PngFile {
                    path: "value.png".to_owned(),
                },
            },
            std::time::Instant::now() + std::time::Duration::from_secs(1),
        );
        assert!(
            result
                .expect_err("invalid id should fail")
                .contains("invalid")
        );
    }

    #[test]
    fn host_services_enforce_manifest_permissions() {
        let router = HostServiceRouter {
            launcher: Err("launcher unavailable".to_owned()),
            clipboard: Err("clipboard unavailable".to_owned()),
            payload_root: Ok(std::env::temp_dir()),
            permissions: Default::default(),
        };
        let error = router
            .submit(
                "com.nanika.test",
                HostServiceRequest::WriteClipboard {
                    content: ClipboardContent::Text {
                        value: "value".to_owned(),
                    },
                },
                std::time::Instant::now() + std::time::Duration::from_secs(1),
            )
            .expect_err("missing permission should fail");
        assert!(error.contains("clipboard.write"));
    }
}
