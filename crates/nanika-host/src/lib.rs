//! Host-owned process supervision boundary.

mod ui;

pub use ui::HostApp;

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nanika_protocol::{FrameError, Message, PROTOCOL_NAME, read_frame, write_frame};

/// Fixed bounds applied to one extension process.
#[derive(Debug, Clone)]
pub struct ExtensionLimits {
    pub handshake_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub frame_queue_capacity: usize,
    pub stderr_tail_bytes: usize,
    pub max_restarts: u32,
}

impl Default for ExtensionLimits {
    fn default() -> Self {
        Self {
            handshake_timeout: Duration::from_secs(2),
            shutdown_timeout: Duration::from_secs(2),
            frame_queue_capacity: 32,
            stderr_tail_bytes: 64 * 1024,
            max_restarts: 3,
        }
    }
}

/// Failures raised by the extension process supervisor.
#[derive(Debug)]
pub enum SupervisorError {
    Io(io::Error),
    Protocol(FrameError),
    Timeout(&'static str),
    ChannelClosed,
    UnexpectedMessage(String),
    RestartLimit,
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Protocol(error) => write!(formatter, "protocol error: {error}"),
            Self::Timeout(operation) => write!(formatter, "extension timed out during {operation}"),
            Self::ChannelClosed => write!(formatter, "extension protocol channel closed"),
            Self::UnexpectedMessage(message) => write!(formatter, "unexpected message: {message}"),
            Self::RestartLimit => write!(formatter, "extension restart limit reached"),
        }
    }
}

impl std::error::Error for SupervisorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::Timeout(_)
            | Self::ChannelClosed
            | Self::UnexpectedMessage(_)
            | Self::RestartLimit => None,
        }
    }
}

impl From<io::Error> for SupervisorError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FrameError> for SupervisorError {
    fn from(error: FrameError) -> Self {
        Self::Protocol(error)
    }
}

#[derive(Debug, Clone)]
struct ExtensionCommand {
    program: PathBuf,
    arguments: Vec<OsString>,
}

/// A supervised extension child process using the universal protocol.
pub struct ExtensionProcess {
    command: ExtensionCommand,
    limits: ExtensionLimits,
    restart_count: u32,
    child: Child,
    input: Option<BufWriter<ChildStdin>>,
    output: Option<Receiver<Result<Option<Message>, FrameError>>>,
    stderr_tail: Arc<Mutex<VecDeque<u8>>>,
    reader_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
}

impl ExtensionProcess {
    /// Start an extension executable with protocol-owned standard streams.
    pub fn spawn(program: impl AsRef<Path>) -> io::Result<Self> {
        Self::spawn_with(
            program,
            std::iter::empty::<OsString>(),
            ExtensionLimits::default(),
        )
    }

    /// Start an extension with structured arguments and explicit limits.
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

        let input = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("extension stdin was not piped"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("extension stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("extension stderr was not piped"))?;

        let (sender, receiver) = mpsc::sync_channel(limits.frame_queue_capacity.max(1));
        let reader_thread = std::thread::Builder::new()
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
            })?;

        let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));
        let stderr_output = Arc::clone(&stderr_tail);
        let stderr_limit = limits.stderr_tail_bytes;
        let stderr_thread = std::thread::Builder::new()
            .name("nanika-extension-stderr".to_owned())
            .spawn(move || drain_stderr(stderr, &stderr_output, stderr_limit))?;

        Ok(Self {
            command,
            limits,
            restart_count,
            child,
            input: Some(BufWriter::new(input)),
            output: Some(receiver),
            stderr_tail,
            reader_thread: Some(reader_thread),
            stderr_thread: Some(stderr_thread),
        })
    }

    /// Send one protocol message to the extension.
    pub fn send(&mut self, message: &Message) -> Result<(), SupervisorError> {
        let input = self.input.as_mut().ok_or(SupervisorError::ChannelClosed)?;
        write_frame(input, message).map_err(SupervisorError::Protocol)
    }

    /// Receive one protocol message without a deadline.
    pub fn receive(&mut self) -> Result<Option<Message>, SupervisorError> {
        self.output
            .as_ref()
            .ok_or(SupervisorError::ChannelClosed)?
            .recv()
            .map_err(|_| SupervisorError::ChannelClosed)?
            .map_err(SupervisorError::Protocol)
    }

    /// Receive one protocol message before the supplied deadline.
    pub fn receive_timeout(
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

    /// Perform the initial protocol handshake within the configured deadline.
    pub fn initialize(&mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        let request_id = request_id.into();
        self.send(&Message::Initialize {
            request_id: request_id.clone(),
            protocol: PROTOCOL_NAME.to_owned(),
        })?;

        match self.receive_timeout(self.limits.handshake_timeout, "initialization")? {
            Some(Message::Initialized {
                request_id: response_id,
                protocol,
            }) if response_id == request_id && protocol == PROTOCOL_NAME => Ok(()),
            Some(message) => Err(SupervisorError::UnexpectedMessage(format!("{message:?}"))),
            None => Err(SupervisorError::ChannelClosed),
        }
    }

    /// Return the child exit status when it has exited, without blocking.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    /// Restart an extension within its fixed restart budget and repeat the handshake.
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

    /// Restart and initialize only when the child has already exited.
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

    /// Return a bounded, lossy UTF-8 tail of extension stderr.
    pub fn stderr_tail(&self) -> String {
        let mut bytes = self
            .stderr_tail
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        String::from_utf8_lossy(bytes.make_contiguous()).into_owned()
    }

    /// Terminate the child and wait for the operating system to reap it.
    pub fn terminate(&mut self) -> io::Result<()> {
        self.input.take();
        self.output.take();
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        self.child.wait()?;
        self.join_threads();
        Ok(())
    }

    /// Request an orderly shutdown, enforce its deadline, and reap the child.
    pub fn shutdown(mut self, request_id: impl Into<String>) -> Result<(), SupervisorError> {
        let request_id = request_id.into();
        self.send(&Message::Shutdown {
            request_id: request_id.clone(),
        })?;

        match self.receive_timeout(self.limits.shutdown_timeout, "shutdown acknowledgement")? {
            Some(Message::ShutdownAck {
                request_id: response_id,
            }) if response_id == request_id => {}
            Some(message) => {
                return Err(SupervisorError::UnexpectedMessage(format!("{message:?}")));
            }
            None => return Err(SupervisorError::ChannelClosed),
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
