use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, BufReader, BufWriter, Read};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nanika_protocol::{FrameError, Message, PROTOCOL_NAME, read_frame, write_frame};

use crate::{ExtensionCommand, ExtensionLimits, SupervisorError};

/// A supervised extension child process using the universal protocol.
pub struct ExtensionProcess {
    command: ExtensionCommand,
    limits: ExtensionLimits,
    restart_count: u32,
    initialized: bool,
    child: Child,
    input: Option<BufWriter<ChildStdin>>,
    output: Option<Receiver<Result<Option<Message>, FrameError>>>,
    stderr_tail: Arc<Mutex<VecDeque<u8>>>,
    reader_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
}

impl ExtensionProcess {
    pub fn spawn(program: impl AsRef<Path>) -> io::Result<Self> {
        Self::spawn_with(
            program,
            std::iter::empty::<OsString>(),
            ExtensionLimits::default(),
        )
    }

    pub fn spawn_with(
        program: impl AsRef<Path>,
        arguments: impl IntoIterator<Item = OsString>,
        limits: ExtensionLimits,
    ) -> io::Result<Self> {
        let command = ExtensionCommand {
            program: program.as_ref().to_path_buf(),
            arguments: arguments.into_iter().collect(),
        };
        Self::start(command, limits, 0)
    }

    fn start(
        command: ExtensionCommand,
        limits: ExtensionLimits,
        restart_count: u32,
    ) -> io::Result<Self> {
        let mut child = Command::new(&command.program)
            .args(&command.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let Some(input) = child.stdin.take() else {
            cleanup_failed_spawn(&mut child);
            return Err(io::Error::other("extension stdin was not piped"));
        };
        let Some(output) = child.stdout.take() else {
            cleanup_failed_spawn(&mut child);
            return Err(io::Error::other("extension stdout was not piped"));
        };
        let Some(stderr) = child.stderr.take() else {
            cleanup_failed_spawn(&mut child);
            return Err(io::Error::other("extension stderr was not piped"));
        };

        let (sender, receiver) = mpsc::sync_channel(limits.frame_queue_capacity.max(1));
        let reader_thread = match std::thread::Builder::new()
            .name("nanika-extension-protocol".to_owned())
            .spawn(move || {
                let mut reader = BufReader::new(output);
                loop {
                    let frame = read_frame(&mut reader);
                    let finished = !matches!(frame, Ok(Some(_)));
                    if sender.send(frame).is_err() || finished {
                        break;
                    }
                }
            }) {
            Ok(thread) => thread,
            Err(error) => {
                cleanup_failed_spawn(&mut child);
                return Err(error);
            }
        };

        let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_output = Arc::clone(&stderr_tail);
        let stderr_limit = limits.stderr_tail_bytes;
        let stderr_thread = match std::thread::Builder::new()
            .name("nanika-extension-stderr".to_owned())
            .spawn(move || drain_stderr(stderr, &stderr_output, stderr_limit))
        {
            Ok(thread) => thread,
            Err(error) => {
                drop(receiver);
                cleanup_failed_spawn(&mut child);
                let _ = reader_thread.join();
                return Err(error);
            }
        };

        Ok(Self {
            command,
            limits,
            restart_count,
            initialized: false,
            child,
            input: Some(BufWriter::new(input)),
            output: Some(receiver),
            stderr_tail,
            reader_thread: Some(reader_thread),
            stderr_thread: Some(stderr_thread),
        })
    }

    fn send(&mut self, message: &Message) -> Result<(), SupervisorError> {
        let input = self.input.as_mut().ok_or(SupervisorError::ChannelClosed)?;
        write_frame(input, message).map_err(SupervisorError::Protocol)
    }

    fn receive_timeout(
        &mut self,
        timeout: Duration,
        operation: &'static str,
    ) -> Result<Option<Message>, SupervisorError> {
        match self
            .output
            .as_ref()
            .ok_or(SupervisorError::ChannelClosed)?
            .recv_timeout(timeout)
        {
            Ok(frame) => frame.map_err(SupervisorError::Protocol),
            Err(RecvTimeoutError::Timeout) => Err(SupervisorError::Timeout(operation)),
            Err(RecvTimeoutError::Disconnected) => Err(SupervisorError::ChannelClosed),
        }
    }

    pub fn initialize(&mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        if self.initialized {
            return Ok(());
        }
        let request_id = request_id.into();
        if let Err(error) = self.send(&Message::Initialize {
            request_id: request_id.clone(),
            protocol: PROTOCOL_NAME.to_owned(),
        }) {
            let _ = self.terminate();
            return Err(error);
        }
        let result = match self.receive_timeout(self.limits.handshake_timeout, "initialization") {
            Err(error) => Err(error),
            Ok(Some(Message::Initialized {
                request_id: response_id,
                protocol,
            })) if response_id == request_id && protocol == PROTOCOL_NAME => {
                self.initialized = true;
                Ok(())
            }
            Ok(Some(message)) => Err(SupervisorError::UnexpectedMessage(format!("{message:?}"))),
            Ok(None) => Err(SupervisorError::ChannelClosed),
        };
        if result.is_err() {
            let _ = self.terminate();
        }
        result
    }

    pub fn query(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        query: impl Into<String>,
        timeout: Duration,
    ) -> Result<Vec<nanika_protocol::Candidate>, SupervisorError> {
        let mut latest = Vec::new();
        self.query_incremental(
            request_id,
            generation,
            query,
            timeout,
            |entries| {
                latest = entries;
                Ok(())
            },
            || false,
        )?;
        Ok(latest)
    }

    pub fn invoke(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.invoke_cancellable(request_id, generation, entry_id, action_id, || false)
    }

    pub(crate) fn invoke_cancellable(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<(), SupervisorError> {
        self.ensure_initialized()?;
        let request_id = request_id.into();
        self.send(&Message::Invoke {
            request_id: request_id.clone(),
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
        })?;
        let deadline = Instant::now() + self.limits.action_timeout;
        loop {
            if should_cancel() {
                self.terminate()?;
                return Err(SupervisorError::Cancelled("action"));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SupervisorError::Timeout("action result"));
            }
            let wait = remaining.min(Duration::from_millis(25));
            let message = match self.receive_timeout(wait, "action result") {
                Ok(message) => message,
                Err(SupervisorError::Timeout(_)) => continue,
                Err(error) => return Err(error),
            };
            match message {
                Some(Message::Result {
                    request_id: response_id,
                    generation: response_generation,
                }) if response_id == request_id && response_generation == generation => {
                    return Ok(());
                }
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) if response_id.as_deref().is_none_or(|id| id == request_id) => {
                    return Err(SupervisorError::UnexpectedMessage(format!(
                        "extension action failed with {code}: {message}"
                    )));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
    }

    pub(crate) fn query_incremental(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        query: impl Into<String>,
        timeout: Duration,
        mut publish: impl FnMut(Vec<nanika_protocol::Candidate>) -> Result<(), SupervisorError>,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        self.ensure_initialized()?;
        let request_id = request_id.into();
        self.send(&Message::Query {
            request_id: request_id.clone(),
            generation,
            query: query.into(),
        })?;
        let deadline = Instant::now() + timeout;
        loop {
            if should_cancel() {
                self.send(&Message::Cancel {
                    request_id: request_id.clone(),
                    generation,
                })?;
                return Ok(false);
            }
            let now = Instant::now();
            if now >= deadline {
                let _ = self.send(&Message::Cancel {
                    request_id: request_id.clone(),
                    generation,
                });
                return Err(SupervisorError::Timeout("query snapshot"));
            }
            let wait = deadline
                .saturating_duration_since(now)
                .min(Duration::from_millis(25));
            let message = match self.receive_timeout(wait, "query snapshot") {
                Ok(message) => message,
                Err(SupervisorError::Timeout(_)) => continue,
                Err(error) => return Err(error),
            };
            match message {
                Some(Message::Snapshot {
                    request_id: response_id,
                    generation: response_generation,
                    complete,
                    entries,
                }) if response_id == request_id && response_generation == generation => {
                    publish(entries)?;
                    if complete {
                        return Ok(true);
                    }
                }
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) if response_id.as_deref().is_none_or(|id| id == request_id) => {
                    return Err(SupervisorError::UnexpectedMessage(format!(
                        "extension query failed with {code}: {message}"
                    )));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    pub fn restart(&mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        if self.restart_count >= self.limits.max_restarts {
            return Err(SupervisorError::RestartLimit);
        }
        self.terminate()?;
        let replacement = Self::start(
            self.command.clone(),
            self.limits.clone(),
            self.restart_count + 1,
        )?;
        *self = replacement;
        self.initialize(request_id)
    }

    pub fn recover_if_exited(
        &mut self,
        request_id: impl Into<String>,
    ) -> Result<bool, SupervisorError> {
        if self.child.try_wait()?.is_none() {
            return Ok(false);
        }
        self.restart(request_id)?;
        Ok(true)
    }

    pub fn stderr_tail(&self) -> String {
        let mut bytes = self
            .stderr_tail
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        String::from_utf8_lossy(bytes.make_contiguous()).into_owned()
    }

    pub fn terminate(&mut self) -> io::Result<()> {
        self.initialized = false;
        self.input.take();
        self.output.take();
        let mut first_error = None;
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                if let Err(error) = self.child.kill() {
                    first_error = Some(error);
                }
            }
            Err(error) => {
                first_error = Some(error);
                let _ = self.child.kill();
            }
        }
        if let Err(error) = self.child.wait()
            && first_error.is_none()
        {
            first_error = Some(error);
        }
        self.join_threads();
        first_error.map_or(Ok(()), Err)
    }

    pub fn shutdown(mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        let request_id = request_id.into();
        self.send(&Message::Shutdown {
            request_id: request_id.clone(),
        })?;
        let acknowledgement_deadline = Instant::now() + self.limits.shutdown_timeout;
        loop {
            let remaining = acknowledgement_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SupervisorError::Timeout("shutdown acknowledgement"));
            }
            match self.receive_timeout(remaining, "shutdown acknowledgement")? {
                Some(Message::ShutdownAck {
                    request_id: response_id,
                }) if response_id == request_id => break,
                Some(_) => continue,
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
        let deadline = Instant::now() + self.limits.shutdown_timeout;
        while self.child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                self.terminate()?;
                return Err(SupervisorError::Timeout("process exit"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        self.input.take();
        self.output.take();
        self.join_threads();
        Ok(())
    }

    fn join_threads(&mut self) {
        if let Some(thread) = self.reader_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.stderr_thread.take() {
            let _ = thread.join();
        }
    }

    fn ensure_initialized(&self) -> Result<(), SupervisorError> {
        if self.initialized {
            Ok(())
        } else {
            Err(SupervisorError::UnexpectedMessage(
                "extension is not initialized".to_owned(),
            ))
        }
    }
}

impl Drop for ExtensionProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

fn drain_stderr(mut stderr: impl Read, output: &Arc<Mutex<VecDeque<u8>>>, byte_limit: usize) {
    let mut chunk = [0; 4096];
    loop {
        let read = match stderr.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        if byte_limit == 0 {
            continue;
        }
        let mut tail = output.lock().unwrap_or_else(|error| error.into_inner());
        tail.extend(&chunk[..read]);
        while tail.len() > byte_limit {
            tail.pop_front();
        }
    }
}

fn cleanup_failed_spawn(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
