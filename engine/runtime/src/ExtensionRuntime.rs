use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use nanika_extension_package::ExtensionProtocol;
use nanika_protocol::{Candidate, ExtensionConfiguration};

use crate::{
    AcpExtensionProcess, ExtensionInterruption, ExtensionLimits, ExtensionProcess,
    ExtensionRuntimeInvocation, HostServiceHandler, SupervisorError,
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
    pub(crate) fn set_shutdown_signal(&mut self, signal: Arc<AtomicBool>) {
        match self {
            Self::Nanika(process) => process.set_shutdown_signal(signal),
            Self::Acp(process) => process.set_shutdown_signal(signal),
        }
    }

    pub fn spawn_with(
        extension_id: impl Into<String>,
        protocol: ExtensionProtocol,
        program: impl AsRef<Path>,
        arguments: impl IntoIterator<Item = OsString>,
        limits: ExtensionLimits,
    ) -> io::Result<Self> {
        Self::spawn_with_configuration(
            extension_id,
            protocol,
            program,
            arguments,
            limits,
            Default::default(),
        )
    }

    pub fn spawn_with_configuration(
        extension_id: impl Into<String>,
        protocol: ExtensionProtocol,
        program: impl AsRef<Path>,
        arguments: impl IntoIterator<Item = OsString>,
        limits: ExtensionLimits,
        configuration: ExtensionConfiguration,
    ) -> io::Result<Self> {
        let extension_id = extension_id.into();
        match protocol {
            ExtensionProtocol::Nanika {
                protocol_version: 1,
            } => ExtensionProcess::spawn_with(program, arguments, limits).map(Self::Nanika),
            ExtensionProtocol::Acp {
                protocol_version: 1,
            } => AcpExtensionProcess::spawn_with_configuration(
                extension_id,
                program,
                arguments,
                limits,
                configuration,
            )
            .map(Self::Acp),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported extension protocol",
            )),
        }
    }

    pub(crate) fn set_candidate_notifier(&mut self, notify: Arc<dyn Fn() + Send + Sync>) {
        if let Self::Nanika(process) = self {
            process.set_candidate_notifier(notify);
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
        self.initialize_with_configuration(request_id, ExtensionConfiguration::default())
    }

    pub fn initialize_with_configuration(
        &mut self,
        request_id: impl Into<String>,
        configuration: ExtensionConfiguration,
    ) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => {
                process.initialize_with_configuration(request_id, configuration)
            }
            Self::Acp(process) => process.initialize(),
        }
    }

    pub fn supports_live_configuration(&self) -> bool {
        matches!(self, Self::Nanika(_))
    }

    pub fn apply_configuration(
        &mut self,
        request_id: impl Into<String>,
        configuration: ExtensionConfiguration,
    ) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => process.apply_configuration(request_id, configuration),
            Self::Acp(_) => Err(SupervisorError::UnexpectedMessage(
                "ACP does not support live configuration updates".to_owned(),
            )),
        }
    }

    pub(crate) fn refresh_cancellable(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => {
                process.refresh_cancellable(request_id, generation, should_cancel)
            }
            Self::Acp(_) => Ok(!should_cancel()),
        }
    }

    pub fn query_incremental(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        query: impl Into<String>,
        mut publish: impl FnMut(Vec<Candidate>) -> Result<(), SupervisorError>,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        match self {
            Self::Nanika(process) => {
                process.query_incremental(request_id, generation, query, publish, should_cancel)
            }
            Self::Acp(_) if should_cancel() => Ok(false),
            Self::Acp(process) => {
                let query = query.into();
                let entries = if acp_prompt(process.extension_id(), &query).is_some() {
                    vec![Candidate {
                        entry_id: "prompt".to_owned(),
                        title: format!("Ask {}", process.extension_id()),
                        subtitle: Some("AI Command".to_owned()),
                        action_id: "prompt".to_owned(),
                        aliases: vec![query],
                        icon: None,
                        command_icon: None,
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
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<(nanika_protocol::NavigationEffect, bool), SupervisorError> {
        self.invoke_interruptible(invocation, publish, || {
            if should_cancel() {
                ExtensionInterruption::Cancel
            } else {
                ExtensionInterruption::None
            }
        })
    }

    pub(crate) fn invoke_interruptible(
        &mut self,
        invocation: ExtensionRuntimeInvocation,
        publish: Arc<dyn Fn(String) + Send + Sync>,
        interruption: impl FnMut() -> ExtensionInterruption,
    ) -> Result<(nanika_protocol::NavigationEffect, bool), SupervisorError> {
        match self {
            Self::Nanika(process) => process
                .invoke_interruptible(
                    invocation.request_id,
                    invocation.generation,
                    invocation.entry_id,
                    invocation.action_id,
                    interruption,
                )
                .map(|effect| (effect, false)),
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
                    .prompt_interruptible(prompt, publish, interruption)
                    .map(|()| (nanika_protocol::NavigationEffect::None, true))
            }
        }
    }

    pub fn ensure_running(&mut self) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => process.ensure_running(),
            Self::Acp(process) => process.ensure_running(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn view_event_cancellable(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent,
        should_cancel: impl FnMut() -> bool,
    ) -> Result<
        (
            u64,
            nanika_protocol::NavigationEffect,
            Option<nanika_protocol::View>,
        ),
        SupervisorError,
    > {
        match self {
            Self::Nanika(process) => process.view_event_cancellable(
                request_id,
                generation,
                view_id,
                revision,
                event,
                should_cancel,
            ),
            Self::Acp(_) => Err(SupervisorError::UnexpectedMessage(
                "ACP extensions cannot own host-rendered views".to_owned(),
            )),
        }
    }

    pub(crate) fn close_view(
        &mut self,
        request_id: impl Into<String>,
        view_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        match self {
            Self::Nanika(process) => process.close_view(request_id, view_id),
            Self::Acp(_) => Err(SupervisorError::UnexpectedMessage(
                "ACP extensions cannot own host-rendered views".to_owned(),
            )),
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
