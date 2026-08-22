use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use nanika_platform::{ClipboardService, ProcessLauncher};
use nanika_protocol::{ClipboardContent, HostServiceRequest, HostServiceResponse};
use nanika_storage::is_valid_extension_id;

use crate::HostServiceHandler;

/// Routes typed extension requests to host-owned platform services.
pub struct HostServiceRouter {
    launcher: Result<ProcessLauncher, String>,
    clipboard: Result<ClipboardService, String>,
    payload_root: Result<PathBuf, String>,
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
}

impl HostServiceHandler for HostServiceRouter {
    fn submit(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
        deadline: Instant,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        if !is_valid_extension_id(extension_id) {
            return Err("host service request has an invalid extension id".to_owned());
        }
        match request {
            HostServiceRequest::Launch { descriptor } => {
                self.launcher()?.submit(descriptor, deadline)
            }
            HostServiceRequest::WriteClipboard { content } => {
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
        };
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
}
