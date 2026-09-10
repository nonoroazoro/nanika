use std::collections::VecDeque;
use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use agent_client_protocol::role::HasPeer;
use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        CancelNotification, ContentBlock, ContentChunk, InitializeRequest, SessionNotification,
        SessionUpdate, StopReason,
    },
};
use agent_client_protocol::util::MatchDispatch;
use agent_client_protocol::{ActiveSession, Agent, Client, ConnectionTo, Lines, SessionMessage};
use futures_lite::future;

use crate::{
    AcpConnectionContext, AcpExtensionCommand, ExtensionCommand, ExtensionInterruption,
    ExtensionLimits, ExtensionProcessTree, SupervisorError, configure_extension_command,
    drain_stderr, incoming_lines, outgoing_lines, terminate_child,
};

const ACP_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// A supervised stable ACP v1 extension child process.
pub struct AcpExtensionProcess {
    extension_id: String,
    initialized: bool,
    shutdown_requested: Arc<AtomicBool>,
    commands: Option<async_channel::Sender<AcpExtensionCommand>>,
    shutdown: Option<async_channel::Sender<()>>,
    ready: Receiver<Result<(), String>>,
    thread: Option<JoinHandle<()>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl AcpExtensionProcess {
    pub fn spawn_with(
        extension_id: impl Into<String>,
        program: impl AsRef<Path>,
        arguments: impl IntoIterator<Item = OsString>,
        limits: ExtensionLimits,
    ) -> io::Result<Self> {
        let command = ExtensionCommand {
            program: program.as_ref().to_path_buf(),
            arguments: arguments.into_iter().collect(),
        };
        let working_directory = command
            .program
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| io::Error::other("ACP extension has no working directory"))?;
        Self::start(extension_id.into(), command, working_directory, limits)
    }

    fn start(
        extension_id: String,
        command: ExtensionCommand,
        working_directory: PathBuf,
        _limits: ExtensionLimits,
    ) -> io::Result<Self> {
        let arguments = command
            .arguments
            .iter()
            .map(|argument| {
                argument.clone().into_string().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "ACP extension arguments must be valid UTF-8",
                    )
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let (commands, command_receiver) = async_channel::bounded(1);
        let (shutdown, shutdown_receiver) = async_channel::bounded(1);
        let (ready_sender, ready) = mpsc::sync_channel(1);
        let ready_reported = Arc::new(AtomicBool::new(false));
        let thread_ready_reported = Arc::clone(&ready_reported);
        let fallback_ready_sender = ready_sender.clone();
        let last_error = Arc::new(Mutex::new(None));
        let thread_error = Arc::clone(&last_error);
        let thread_command = command.clone();
        let thread_working_directory = working_directory.clone();
        let thread_extension_id = extension_id.clone();
        let thread = std::thread::Builder::new()
            .name(format!("nanika-acp-extension-{extension_id}"))
            .spawn(move || {
                let result = async_io::block_on(run_connection(AcpConnectionContext {
                    extension_id: thread_extension_id,
                    command: thread_command,
                    arguments,
                    working_directory: thread_working_directory,
                    commands: command_receiver,
                    shutdown: shutdown_receiver,
                    ready: ready_sender,
                    ready_reported,
                }));
                if let Err(error) = result {
                    if !thread_ready_reported.swap(true, Ordering::AcqRel) {
                        let _ = fallback_ready_sender.try_send(Err(error.to_string()));
                    }
                    *thread_error
                        .lock()
                        .unwrap_or_else(|error| error.into_inner()) = Some(error.to_string());
                }
            })?;
        Ok(Self {
            extension_id,
            initialized: false,
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            commands: Some(commands),
            shutdown: Some(shutdown),
            ready,
            thread: Some(thread),
            last_error,
        })
    }

    pub(crate) fn set_shutdown_signal(&mut self, signal: Arc<AtomicBool>) {
        self.shutdown_requested = signal;
    }

    pub fn initialize(&mut self) -> Result<(), SupervisorError> {
        if self.initialized {
            return Ok(());
        }
        let result = loop {
            if self.shutdown_requested.load(Ordering::Acquire) {
                self.terminate()?;
                break Err(SupervisorError::Cancelled("ACP initialization"));
            }
            match self.ready.recv_timeout(ACP_POLL_INTERVAL) {
                Ok(Ok(())) => break Ok(()),
                Ok(Err(error)) => break Err(SupervisorError::UnexpectedMessage(error)),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break Err(SupervisorError::ChannelClosed),
            }
        };
        if result.is_ok() {
            self.initialized = true;
        } else {
            let _ = self.terminate();
        }
        result
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn prompt_cancellable(
        &mut self,
        prompt: impl Into<String>,
        publish: Arc<dyn Fn(String) + Send + Sync>,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<(), SupervisorError> {
        self.prompt_interruptible(prompt, publish, || {
            if should_cancel() {
                ExtensionInterruption::Cancel
            } else {
                ExtensionInterruption::None
            }
        })
    }

    pub(crate) fn prompt_interruptible(
        &mut self,
        prompt: impl Into<String>,
        publish: Arc<dyn Fn(String) + Send + Sync>,
        mut interruption: impl FnMut() -> ExtensionInterruption,
    ) -> Result<(), SupervisorError> {
        if !self.initialized {
            return Err(SupervisorError::UnexpectedMessage(
                "ACP extension is not initialized".to_owned(),
            ));
        }
        let (response_sender, response) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.commands
            .as_ref()
            .ok_or(SupervisorError::ChannelClosed)?
            .send_blocking(AcpExtensionCommand::Prompt {
                prompt: prompt.into(),
                cancelled: Arc::clone(&cancelled),
                publish,
                response: response_sender,
            })
            .map_err(|_| SupervisorError::ChannelClosed)?;
        let mut cancellation_requested = false;
        loop {
            match interruption() {
                ExtensionInterruption::None => {}
                ExtensionInterruption::Cancel if !cancellation_requested => {
                    cancelled.store(true, Ordering::Release);
                    cancellation_requested = true;
                }
                ExtensionInterruption::Cancel => {}
                ExtensionInterruption::Terminate => {
                    self.terminate()?;
                    return Err(SupervisorError::Cancelled("ACP prompt"));
                }
            }
            match response.recv_timeout(ACP_POLL_INTERVAL) {
                Ok(Ok(StopReason::Cancelled)) => {
                    return Err(SupervisorError::Cancelled("ACP prompt"));
                }
                Ok(Ok(_)) => return Ok(()),
                Ok(Err(error)) => return Err(SupervisorError::UnexpectedMessage(error)),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(SupervisorError::ChannelClosed);
                }
            }
        }
    }

    pub fn ensure_running(&self) -> Result<(), SupervisorError> {
        if self.thread.is_some() && !self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
            return Ok(());
        }
        let detail = self.last_error().unwrap_or_else(|| {
            "ACP extension process exited without reporting an error".to_owned()
        });
        Err(SupervisorError::UnexpectedMessage(detail))
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn terminate(&mut self) -> io::Result<()> {
        self.initialized = false;
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.try_send(());
            shutdown.close();
        }
        if let Some(commands) = self.commands.take() {
            commands.close();
        }
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| io::Error::other("ACP extension thread panicked"))?;
        }
        Ok(())
    }

    pub fn shutdown(mut self) -> Result<(), SupervisorError> {
        let (response_sender, response) = mpsc::sync_channel(1);
        if let Some(commands) = &self.commands {
            commands
                .send_blocking(AcpExtensionCommand::Shutdown {
                    response: response_sender,
                })
                .map_err(|_| SupervisorError::ChannelClosed)?;
            match response.recv() {
                Ok(()) => {}
                Err(_) => {
                    let _ = self.terminate();
                    return Err(SupervisorError::ChannelClosed);
                }
            }
        }
        self.terminate()?;
        Ok(())
    }
}

impl Drop for AcpExtensionProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

async fn run_connection(context: AcpConnectionContext) -> agent_client_protocol::Result<()> {
    let mut command = std::process::Command::new(context.command.program);
    command.args(context.arguments);
    configure_extension_command(&mut command);
    let mut command = async_process::Command::from(command);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| agent_client_protocol::util::internal_error(error.to_string()))?;
    let process_tree = match ExtensionProcessTree::attach_async(&child) {
        Ok(process_tree) => process_tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.status().await;
            return Err(agent_client_protocol::util::internal_error(format!(
                "failed to contain ACP extension process: {error}"
            )));
        }
    };
    let (Some(stdin), Some(stdout), Some(stderr)) =
        (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
        let _ = terminate_child(&mut child, &process_tree).await;
        return Err(agent_client_protocol::util::internal_error(
            "ACP extension stdio was not piped",
        ));
    };
    let shutdown = context.shutdown.clone();
    let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));
    let drain_tail = Arc::clone(&stderr_tail);
    let stderr_extension_id = context.extension_id.clone();
    let connection = Client.builder().name("nanika").connect_with(
        Lines::new(outgoing_lines(stdin), incoming_lines(stdout)),
        |connection: ConnectionTo<Agent>| async move {
            let initialized = cancel_on_shutdown(
                connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task(),
                context.shutdown.clone(),
            )
            .await?;
            if initialized.protocol_version != ProtocolVersion::V1 {
                let message = format!(
                    "ACP agent selected unsupported protocol version {}",
                    initialized.protocol_version
                );
                context.ready_reported.store(true, Ordering::Release);
                let _ = context.ready.send(Err(message.clone()));
                return Err(agent_client_protocol::util::internal_error(message));
            }
            let mut session = cancel_on_shutdown(
                connection
                    .build_session(context.working_directory)
                    .block_task()
                    .start_session(),
                context.shutdown.clone(),
            )
            .await?;
            context.ready_reported.store(true, Ordering::Release);
            let _ = context.ready.send(Ok(()));
            loop {
                let command = future::race(
                    async {
                        context.commands.recv().await.map_err(|_| {
                            agent_client_protocol::util::internal_error(
                                "ACP command channel closed",
                            )
                        })
                    },
                    async {
                        context.shutdown.recv().await.map_err(|_| {
                            agent_client_protocol::util::internal_error(
                                "ACP shutdown channel closed",
                            )
                        })?;
                        Err(agent_client_protocol::util::internal_error(
                            "ACP extension shutting down",
                        ))
                    },
                )
                .await;
                match command {
                    Ok(AcpExtensionCommand::Prompt {
                        prompt,
                        cancelled,
                        publish,
                        response,
                    }) => {
                        let result = run_prompt(&mut session, prompt, cancelled, publish).await;
                        let _ = response.send(result.map_err(|error| error.to_string()));
                    }
                    Ok(AcpExtensionCommand::Shutdown { response }) => {
                        let _ = response.send(());
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                }
            }
        },
    );
    let result = future::race(connection, async move {
        let _ = shutdown.recv().await;
        Err(agent_client_protocol::util::internal_error(
            "ACP extension shutting down",
        ))
    });
    let result = future::race(result, async move {
        drain_stderr(stderr, drain_tail, stderr_extension_id).await;
        future::pending::<agent_client_protocol::Result<()>>().await
    })
    .await;
    let cleanup = terminate_child(&mut child, &process_tree).await;
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => {
            let stderr = bounded_tail_string(&stderr_tail);
            if stderr.is_empty() {
                Err(error)
            } else {
                Err(agent_client_protocol::util::internal_error(format!(
                    "{error}; stderr: {stderr}"
                )))
            }
        }
        (_, Err(error)) => Err(agent_client_protocol::util::internal_error(format!(
            "failed to terminate ACP extension: {error}"
        ))),
    }
}

fn bounded_tail_string(tail: &Mutex<VecDeque<u8>>) -> String {
    let mut tail = tail.lock().unwrap_or_else(|error| error.into_inner());
    String::from_utf8_lossy(tail.make_contiguous()).into_owned()
}

async fn cancel_on_shutdown<T>(
    operation: impl Future<Output = agent_client_protocol::Result<T>>,
    shutdown: async_channel::Receiver<()>,
) -> agent_client_protocol::Result<T> {
    future::race(operation, async move {
        let _ = shutdown.recv().await;
        Err(agent_client_protocol::util::internal_error(
            "ACP extension shutting down",
        ))
    })
    .await
}

async fn run_prompt<Link>(
    session: &mut ActiveSession<'_, Link>,
    prompt: String,
    cancelled: Arc<AtomicBool>,
    publish: Arc<dyn Fn(String) + Send + Sync>,
) -> agent_client_protocol::Result<StopReason>
where
    Link: HasPeer<Agent>,
{
    session.send_prompt(prompt)?;
    let mut cancellation_sent = false;
    loop {
        if cancelled.load(Ordering::Acquire) && !cancellation_sent {
            session.connection().send_notification_to(
                Agent,
                CancelNotification::new(session.session_id().clone()),
            )?;
            cancellation_sent = true;
        }
        let update = future::race(async { Some(session.read_update().await) }, async {
            async_io::Timer::after(ACP_POLL_INTERVAL).await;
            None
        })
        .await;
        let Some(update) = update else {
            continue;
        };
        match update? {
            SessionMessage::SessionMessage(dispatch) => {
                let mut chunk = None;
                MatchDispatch::new(dispatch)
                    .if_notification(async |notification: SessionNotification| {
                        if let SessionUpdate::AgentMessageChunk(ContentChunk {
                            content: ContentBlock::Text(text),
                            ..
                        }) = notification.update
                        {
                            chunk = Some(text.text);
                        }
                        Ok(())
                    })
                    .await
                    .otherwise_ignore()?;
                if let Some(chunk) = chunk {
                    publish(chunk);
                }
            }
            SessionMessage::StopReason(reason) => return Ok(reason),
            _ => {}
        }
    }
}
