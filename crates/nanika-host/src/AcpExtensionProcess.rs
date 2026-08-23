use std::collections::VecDeque;
use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use agent_client_protocol::role::HasPeer;
use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        CancelNotification, ContentBlock, ContentChunk, InitializeRequest, SessionNotification,
        SessionUpdate,
    },
};
use agent_client_protocol::util::MatchDispatch;
use agent_client_protocol::{
    AcpAgent, AcpAgentConfig, ActiveSession, Agent, Client, ConnectionTo, Lines, SessionMessage,
};
use futures_lite::future;

use crate::{
    AcpConnectionContext, AcpExtensionCommand, ExtensionCommand, ExtensionLimits, SupervisorError,
    drain_stderr, incoming_lines, outgoing_lines, terminate_child,
};

const ACP_OUTPUT_LIMIT: usize = 256 * 1024;
const ACP_POLL_INTERVAL: Duration = Duration::from_millis(25);
const ACP_CANCEL_GRACE: Duration = Duration::from_millis(100);

/// A supervised stable ACP v1 extension child process.
pub struct AcpExtensionProcess {
    extension_id: String,
    command: ExtensionCommand,
    working_directory: PathBuf,
    limits: ExtensionLimits,
    restart_count: u32,
    initialized: bool,
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
        Self::start(extension_id.into(), command, working_directory, limits, 0)
    }

    fn start(
        extension_id: String,
        command: ExtensionCommand,
        working_directory: PathBuf,
        limits: ExtensionLimits,
        restart_count: u32,
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
        let handshake_timeout = limits.handshake_timeout;
        let shutdown_timeout = limits.shutdown_timeout;
        let thread = std::thread::Builder::new()
            .name(format!("nanika-acp-extension-{extension_id}"))
            .spawn(move || {
                let result = async_io::block_on(run_connection(AcpConnectionContext {
                    command: thread_command,
                    arguments,
                    working_directory: thread_working_directory,
                    handshake_timeout,
                    shutdown_timeout,
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
            command,
            working_directory,
            limits,
            restart_count,
            initialized: false,
            commands: Some(commands),
            shutdown: Some(shutdown),
            ready,
            thread: Some(thread),
            last_error,
        })
    }

    pub fn initialize(&mut self) -> Result<(), SupervisorError> {
        if self.initialized {
            return Ok(());
        }
        let result = match self.ready.recv_timeout(self.limits.handshake_timeout) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(SupervisorError::UnexpectedMessage(error)),
            Err(RecvTimeoutError::Timeout) => Err(SupervisorError::Timeout("ACP initialization")),
            Err(RecvTimeoutError::Disconnected) => Err(SupervisorError::ChannelClosed),
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
        let deadline = Instant::now() + self.limits.action_timeout;
        let mut cancellation_requested = false;
        let mut cancellation_deadline = None;
        loop {
            if !cancellation_requested && should_cancel() {
                cancelled.store(true, Ordering::Release);
                cancellation_requested = true;
                cancellation_deadline = Some(Instant::now() + ACP_CANCEL_GRACE);
            }
            let active_deadline = cancellation_deadline.map_or(deadline, |cancellation_deadline| {
                deadline.min(cancellation_deadline)
            });
            let remaining = active_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                cancelled.store(true, Ordering::Release);
                let _ = self.terminate();
                return if cancellation_requested {
                    Err(SupervisorError::Cancelled("ACP prompt"))
                } else {
                    Err(SupervisorError::Timeout("ACP prompt"))
                };
            }
            match response.recv_timeout(remaining.min(ACP_POLL_INTERVAL)) {
                Ok(Ok(_)) if cancellation_requested => {
                    return Err(SupervisorError::Cancelled("ACP prompt"));
                }
                Ok(Ok(())) => return Ok(()),
                Ok(Err(_)) if cancellation_requested => {
                    return Err(SupervisorError::Cancelled("ACP prompt"));
                }
                Ok(Err(error)) => return Err(SupervisorError::UnexpectedMessage(error)),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(SupervisorError::ChannelClosed);
                }
            }
        }
    }

    pub fn recover_if_exited(&mut self) -> Result<bool, SupervisorError> {
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
            self.restart()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn restart(&mut self) -> Result<(), SupervisorError> {
        if self.restart_count >= self.limits.max_restarts {
            return Err(SupervisorError::RestartLimit);
        }
        let extension_id = self.extension_id.clone();
        let command = self.command.clone();
        let working_directory = self.working_directory.clone();
        let limits = self.limits.clone();
        let restart_count = self.restart_count + 1;
        self.terminate()?;
        let mut replacement = Self::start(
            extension_id,
            command,
            working_directory,
            limits,
            restart_count,
        )?;
        replacement.initialize()?;
        *self = replacement;
        Ok(())
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
            match response.recv_timeout(self.limits.shutdown_timeout) {
                Ok(()) => {}
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.terminate();
                    return Err(SupervisorError::Timeout("ACP shutdown"));
                }
                Err(RecvTimeoutError::Disconnected) => {
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
    let agent = AcpAgent::new(AcpAgentConfig::new(context.command.program).args(context.arguments));
    let (stdin, stdout, stderr, mut child) = agent.spawn_process()?;
    let shutdown = context.shutdown.clone();
    let shutdown_timeout = context.shutdown_timeout;
    let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));
    let drain_tail = Arc::clone(&stderr_tail);
    let connection = Client.builder().name("nanika").connect_with(
        Lines::new(outgoing_lines(stdin), incoming_lines(stdout)),
        |connection: ConnectionTo<Agent>| async move {
            let initialized = race_handshake(
                connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task(),
                context.handshake_timeout,
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
            let mut session = race_handshake(
                connection
                    .build_session(context.working_directory)
                    .block_task()
                    .start_session(),
                context.handshake_timeout,
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
        drain_stderr(stderr, drain_tail).await;
        future::pending::<agent_client_protocol::Result<()>>().await
    })
    .await;
    let cleanup = terminate_child(&mut child, shutdown_timeout).await;
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

async fn race_handshake<T>(
    operation: impl Future<Output = agent_client_protocol::Result<T>>,
    timeout: Duration,
    shutdown: async_channel::Receiver<()>,
) -> agent_client_protocol::Result<T> {
    future::race(
        operation,
        future::race(
            async move {
                async_io::Timer::after(timeout).await;
                Err(agent_client_protocol::util::internal_error(
                    "ACP handshake timed out",
                ))
            },
            async move {
                let _ = shutdown.recv().await;
                Err(agent_client_protocol::util::internal_error(
                    "ACP extension shutting down",
                ))
            },
        ),
    )
    .await
}

async fn run_prompt<Link>(
    session: &mut ActiveSession<'_, Link>,
    prompt: String,
    cancelled: Arc<AtomicBool>,
    publish: Arc<dyn Fn(String) + Send + Sync>,
) -> agent_client_protocol::Result<()>
where
    Link: HasPeer<Agent>,
{
    session.send_prompt(prompt)?;
    let mut output_bytes = 0_usize;
    let mut cancellation_sent = false;
    let mut output_limited = false;
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
                if let Some(chunk) = chunk
                    && !output_limited
                {
                    if output_bytes.saturating_add(chunk.len()) > ACP_OUTPUT_LIMIT {
                        output_limited = true;
                        if !cancellation_sent {
                            session.connection().send_notification_to(
                                Agent,
                                CancelNotification::new(session.session_id().clone()),
                            )?;
                            cancellation_sent = true;
                        }
                    } else {
                        output_bytes += chunk.len();
                        publish(chunk);
                    }
                }
            }
            SessionMessage::StopReason(_) => break,
            _ => {}
        }
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(agent_client_protocol::util::internal_error(
            "ACP prompt cancelled",
        ));
    }
    if output_limited {
        return Err(agent_client_protocol::util::internal_error(format!(
            "ACP prompt output exceeds {ACP_OUTPUT_LIMIT} bytes"
        )));
    }
    Ok(())
}
