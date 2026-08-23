use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use nanika_extension_package::ExtensionProtocol;
use nanika_protocol::{Candidate, SettingUpdate, SettingsContribution};

use crate::{
    AcpExtensionProcess, ExtensionLimits, ExtensionProcess, ExtensionRuntimeInvocation,
    HostServiceHandler, SupervisorError,
};

/// Protocol-aware process supervisor shared by built-in and external extensions.
pub enum ExtensionRuntime {
    Nanika(ExtensionProcess),
    Acp(AcpExtensionProcess),
}

impl From<ExtensionProcess> for ExtensionRuntime {
    fn from(process: ExtensionProcess) -> Self {
        Self::Nanika(process)
    }
}

impl ExtensionRuntime {
    pub fn spawn_with(
        extension_id: impl Into<String>,
        protocol: ExtensionProtocol,
        program: impl AsRef<Path>,
        arguments: impl IntoIterator<Item = OsString>,
        limits: ExtensionLimits,
    ) -> io::Result<Self> {
        let extension_id = extension_id.into();
        match protocol {
            ExtensionProtocol::Nanika {
                protocol_version: 1,
            } => ExtensionProcess::spawn_with(program, arguments, limits).map(Self::Nanika),
            ExtensionProtocol::Acp {
                protocol_version: 1,
            } => AcpExtensionProcess::spawn_with(extension_id, program, arguments, limits)
                .map(Self::Acp),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported extension protocol",
            )),
        }
    }

    pub(crate) fn set_host_services(
        &mut self,
        extension_id: String,
        host_services: Arc<dyn HostServiceHandler>,
    ) {
        if let Self::Nanika(process) = self {
            process.set_host_services(extension_id, host_services);
        }
    }

    pub fn initialize(&mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => process.initialize(request_id),
            Self::Acp(process) => process.initialize(),
        }
    }

    pub fn settings(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<SettingsContribution, SupervisorError> {
        match self {
            Self::Nanika(process) => process.settings(request_id),
            Self::Acp(process) => Ok(SettingsContribution {
                title: process.extension_id().to_owned(),
                fields: Vec::new(),
            }),
        }
    }

    pub fn update_settings(
        &mut self,
        request_id: impl Into<String>,
        updates: Vec<SettingUpdate>,
    ) -> Result<SettingsContribution, SupervisorError> {
        match self {
            Self::Nanika(process) => process.update_settings(request_id, updates),
            Self::Acp(process) if updates.is_empty() => Ok(SettingsContribution {
                title: process.extension_id().to_owned(),
                fields: Vec::new(),
            }),
            Self::Acp(_) => Err(SupervisorError::UnexpectedMessage(
                "ACP extension does not contribute settings".to_owned(),
            )),
        }
    }

    pub(crate) fn refresh_cancellable(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        timeout: Duration,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => {
                process.refresh_cancellable(request_id, generation, timeout, should_cancel)
            }
            Self::Acp(_) => Ok(!should_cancel()),
        }
    }

    pub fn query_incremental(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        query: impl Into<String>,
        timeout: Duration,
        mut publish: impl FnMut(Vec<Candidate>) -> Result<(), SupervisorError>,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => process.query_incremental(
                request_id,
                generation,
                query,
                timeout,
                publish,
                should_cancel,
            ),
            Self::Acp(_) if should_cancel() => Ok(false),
            Self::Acp(process) => {
                let query = query.into();
                let entries = if acp_prompt(process.extension_id(), &query).is_some() {
                    vec![Candidate {
                        entry_id: "prompt".to_owned(),
                        title: format!("Ask {}", process.extension_id()),
                        action_id: "prompt".to_owned(),
                        aliases: vec![query],
                    }]
                } else {
                    Vec::new()
                };
                publish(entries)?;
                Ok(true)
            }
        }
    }

    pub fn invoke_cancellable(
        &mut self,
        invocation: ExtensionRuntimeInvocation,
        publish: Arc<dyn Fn(String) + Send + Sync>,
        should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => process
                .invoke_cancellable(
                    invocation.request_id,
                    invocation.generation,
                    invocation.entry_id,
                    invocation.action_id,
                    should_cancel,
                )
                .map(|()| false),
            Self::Acp(process) => {
                if invocation.entry_id != "prompt" || invocation.action_id != "prompt" {
                    return Err(SupervisorError::UnexpectedMessage(
                        "ACP extension received an unknown action".to_owned(),
                    ));
                }
                let prompt = acp_prompt(process.extension_id(), &invocation.query_context)
                    .ok_or_else(|| {
                        SupervisorError::UnexpectedMessage(
                            "ACP prompt does not match the extension activation prefix".to_owned(),
                        )
                    })?;
                process
                    .prompt_cancellable(prompt, publish, should_cancel)
                    .map(|()| true)
            }
        }
    }

    pub fn recover_if_exited(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => process.recover_if_exited(request_id),
            Self::Acp(process) => process.recover_if_exited(),
        }
    }

    pub(crate) fn recover_after_cancellation(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => process.recover_after_cancellation(request_id),
            Self::Acp(process) => process.recover_after_cancellation(),
        }
    }

    pub fn restart(&mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => process.restart(request_id),
            Self::Acp(process) => process.restart(),
        }
    }

    pub fn terminate(&mut self) -> io::Result<()> {
        match self {
            Self::Nanika(process) => process.terminate(),
            Self::Acp(process) => process.terminate(),
        }
    }

    pub fn shutdown(self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => process.shutdown(request_id),
            Self::Acp(process) => process.shutdown(),
        }
    }
}

pub(crate) fn acp_prompt<'a>(extension_id: &str, query: &'a str) -> Option<&'a str> {
    let query = query.trim();
    let activation = query.strip_prefix('@')?;
    let prompt = activation.strip_prefix(extension_id)?;
    if !prompt.starts_with(char::is_whitespace) {
        return None;
    }
    let prompt = prompt.trim_start();
    (!prompt.is_empty()).then_some(prompt)
}
