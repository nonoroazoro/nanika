use nanika_platform::{ExtensionProcessTree, configure_extension_command};

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, BufReader, BufWriter, Read};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use nanika_protocol::{
    ExtensionConfiguration, FrameError, Message, PROTOCOL_NAME, read_frame, write_frame,
};

use crate::{
    ExtensionCommand, ExtensionInterruption, ExtensionLimits, ExtensionNotifier,
    HostServiceHandler, SupervisorError,
};

type ReceivePoll = Option<Option<Message>>;
type ViewInvalidationNotifier = Arc<Mutex<Option<Arc<dyn Fn(String) + Send + Sync>>>>;

pub struct ExtensionProcess {
    candidate_changes: ExtensionNotifier,
    view_invalidations: ViewInvalidationNotifier,
    configuration_reply: Arc<crate::ConfigurationReply>,
    initialized: bool,
    shutdown_requested: Arc<AtomicBool>,
    child: Child,
    process_tree: ExtensionProcessTree,
    input: Option<BufWriter<ChildStdin>>,
    output: Option<Receiver<Result<Option<Message>, FrameError>>>,
    stderr_tail: Arc<Mutex<VecDeque<u8>>>,
    reader_thread: Option<JoinHandle<()>>,
    stderr_thread: Option<JoinHandle<()>>,
    extension_id: Option<String>,
    host_services: Option<Arc<dyn HostServiceHandler>>,
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
        Self::start(command, limits, Arc::new(Mutex::new(None)))
    }

    fn start(
        command: ExtensionCommand,
        limits: ExtensionLimits,
        candidate_changes: ExtensionNotifier,
    ) -> io::Result<Self> {
        let mut process = Command::new(&command.program);
        process
            .args(&command.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_extension_command(&mut process);
        let mut child = process.spawn()?;
        let process_tree = match ExtensionProcessTree::attach_std(&child) {
            Ok(process_tree) => process_tree,
            Err(error) => {
                cleanup_failed_spawn(&mut child);
                return Err(error);
            }
        };
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
        let view_invalidations = Arc::new(Mutex::new(None::<Arc<dyn Fn(String) + Send + Sync>>));
        let changes = Arc::clone(&candidate_changes);
        let invalidations = Arc::clone(&view_invalidations);
        let configuration_reply = Arc::new(crate::ConfigurationReply::default());
        let reader_configuration = Arc::clone(&configuration_reply);
        let reader_thread = match std::thread::Builder::new()
            .name("nanika-extension-protocol".to_owned())
            .spawn(move || {
                let mut reader = BufReader::new(output);
                loop {
                    let frame = read_frame(&mut reader);
                    if reader_configuration.dispatch(&frame) {
                        continue;
                    }
                    if matches!(frame, Ok(Some(Message::CandidatesChanged))) {
                        let notify = changes
                            .lock()
                            .unwrap_or_else(|error| error.into_inner())
                            .clone();
                        if let Some(notify) = notify {
                            notify();
                        }
                        continue;
                    }
                    if let Ok(Some(Message::ViewInvalidated { view_id })) = &frame {
                        let notify = invalidations
                            .lock()
                            .unwrap_or_else(|error| error.into_inner())
                            .clone();
                        if let Some(notify) = notify {
                            notify(view_id.clone());
                        }
                        continue;
                    }
                    let finished = !matches!(frame, Ok(Some(_)));
                    if sender.send(frame).is_err() || finished {
                        break;
                    }
                }
                reader_configuration.fail(SupervisorError::ChannelClosed);
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
        let stderr_source = command.program.display().to_string();
        let stderr_thread = match std::thread::Builder::new()
            .name("nanika-extension-stderr".to_owned())
            .spawn(move || drain_stderr(stderr, &stderr_output, stderr_limit, &stderr_source))
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
            candidate_changes,
            view_invalidations,
            configuration_reply,
            initialized: false,
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            child,
            process_tree,
            input: Some(BufWriter::new(input)),
            output: Some(receiver),
            stderr_tail,
            reader_thread: Some(reader_thread),
            stderr_thread: Some(stderr_thread),
            extension_id: None,
            host_services: None,
        })
    }

    pub(crate) fn set_candidate_notifier(&mut self, notify: Arc<dyn Fn() + Send + Sync>) {
        *self
            .candidate_changes
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notify);
    }

    pub(crate) fn set_view_invalidation_notifier(
        &mut self,
        notify: Arc<dyn Fn(String) + Send + Sync>,
    ) {
        *self
            .view_invalidations
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notify);
    }

    pub(crate) fn set_host_services(
        &mut self,
        extension_id: String,
        host_services: Arc<dyn HostServiceHandler>,
    ) {
        self.extension_id = Some(extension_id);
        self.host_services = Some(host_services);
    }

    pub(crate) fn set_shutdown_signal(&mut self, signal: Arc<AtomicBool>) {
        self.shutdown_requested = signal;
    }

    fn check_shutdown(&mut self) -> Result<(), SupervisorError> {
        if self.shutdown_requested.load(Ordering::Acquire) {
            self.terminate()?;
            return Err(SupervisorError::Cancelled("application shutdown"));
        }
        Ok(())
    }

    fn send(&mut self, message: &Message) -> Result<(), SupervisorError> {
        self.check_shutdown()?;
        let input = self.input.as_mut().ok_or(SupervisorError::ChannelClosed)?;
        write_frame(input, message).map_err(SupervisorError::Protocol)
    }

    pub(crate) fn prepare_entries(
        &mut self,
        generation: u64,
        entry_ids: Vec<String>,
    ) -> Result<(), SupervisorError> {
        self.send(&Message::PrepareEntries {
            generation,
            entry_ids,
        })
    }

    fn poll_receive(&mut self, interval: Duration) -> Result<ReceivePoll, SupervisorError> {
        self.check_shutdown()?;
        match self
            .output
            .as_ref()
            .ok_or(SupervisorError::ChannelClosed)?
            .recv_timeout(interval)
        {
            Ok(frame) => frame.map(Some).map_err(SupervisorError::Protocol),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(SupervisorError::ChannelClosed),
        }
    }

    fn receive(&mut self) -> Result<Option<Message>, SupervisorError> {
        // All protocol waits poll for explicit shutdown here; operations never expire.
        loop {
            if let Some(message) = self.poll_receive(Duration::from_millis(25))? {
                return Ok(message);
            }
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
        if self.initialized {
            return Ok(());
        }
        let request_id = request_id.into();
        if let Err(error) = self.send(&Message::Initialize {
            request_id: request_id.clone(),
            protocol: PROTOCOL_NAME.to_owned(),
            configuration,
        }) {
            let _ = self.terminate();
            return Err(error);
        }
        let result = match self.receive() {
            Err(error) => Err(error),
            Ok(Some(Message::Initialized {
                request_id: response_id,
                protocol,
            })) if response_id == request_id && protocol == PROTOCOL_NAME => {
                self.initialized = true;
                Ok(())
            }
            Ok(Some(Message::Error {
                request_id: response_id,
                code,
                message,
            })) => Err(extension_reported_error(
                "initialize",
                &request_id,
                response_id.as_deref(),
                &code,
                &message,
            )),
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
    ) -> Result<Vec<nanika_protocol::Candidate>, SupervisorError> {
        let mut latest = Vec::new();
        self.query_incremental(
            request_id,
            generation,
            query,
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
            .map(|_| ())
    }

    pub fn refresh(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
    ) -> Result<(), SupervisorError> {
        self.refresh_cancellable(request_id, generation, || false)
            .map(|_| ())
    }

    pub fn apply_configuration(
        &mut self,
        request_id: impl Into<String>,
        configuration: ExtensionConfiguration,
    ) -> Result<(), SupervisorError> {
        self.ensure_initialized()?;
        let request_id = request_id.into();
        self.send(&Message::ConfigurationChanged {
            request_id: request_id.clone(),
            configuration,
        })?;
        self.receive_configuration_applied(request_id)
    }

    pub(crate) fn start_configuration_update(
        &mut self,
        request_id: String,
        configuration: ExtensionConfiguration,
        completion: crate::ConfigurationCompletion,
    ) {
        if let Err(error) = self.ensure_initialized() {
            completion(Err(error));
            return;
        }
        if !self
            .configuration_reply
            .register(request_id.clone(), completion)
        {
            return;
        }
        if let Err(error) = self.send(&Message::ConfigurationChanged {
            request_id,
            configuration,
        }) {
            self.configuration_reply.fail(error);
        }
    }

    pub(crate) fn refresh_cancellable(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<bool, SupervisorError> {
        self.ensure_initialized()?;
        let request_id = request_id.into();
        self.send(&Message::Refresh {
            request_id: request_id.clone(),
            generation,
        })?;
        loop {
            if should_cancel() {
                self.send(&Message::Cancel {
                    request_id: request_id.clone(),
                    generation,
                })?;
                return Ok(false);
            }
            let message = match self.poll_receive(Duration::from_millis(25))? {
                Some(message) => message,
                None => continue,
            };
            match message {
                Some(Message::Refreshed {
                    request_id: response_id,
                    generation: response_generation,
                }) if response_id == request_id && response_generation == generation => {
                    return Ok(true);
                }
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) => {
                    return Err(extension_reported_error(
                        "refresh",
                        &request_id,
                        response_id.as_deref(),
                        &code,
                        &message,
                    ));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
    }

    pub(crate) fn invoke_cancellable(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<nanika_protocol::NavigationEffect, SupervisorError> {
        self.invoke_interruptible(request_id, generation, entry_id, action_id, || {
            if should_cancel() {
                ExtensionInterruption::Cancel
            } else {
                ExtensionInterruption::None
            }
        })
    }

    pub(crate) fn invoke_interruptible(
        &mut self,
        request_id: impl Into<String>,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        mut interruption: impl FnMut() -> ExtensionInterruption,
    ) -> Result<nanika_protocol::NavigationEffect, SupervisorError> {
        if matches!(interruption(), ExtensionInterruption::Cancel) {
            return Err(SupervisorError::Cancelled("action"));
        }
        self.ensure_initialized()?;
        let request_id = request_id.into();
        self.send(&Message::Invoke {
            request_id: request_id.clone(),
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
        })?;
        let mut cancellation_sent = false;
        loop {
            match interruption() {
                ExtensionInterruption::None => {}
                ExtensionInterruption::Cancel if !cancellation_sent => {
                    self.send(&Message::Cancel {
                        request_id: request_id.clone(),
                        generation,
                    })?;
                    cancellation_sent = true;
                }
                ExtensionInterruption::Cancel => {}
                ExtensionInterruption::Terminate => {
                    self.terminate()?;
                    return Err(SupervisorError::Cancelled("action"));
                }
            }
            let message = match self.poll_receive(Duration::from_millis(25))? {
                Some(message) => message,
                None => continue,
            };
            match message {
                Some(Message::Result {
                    request_id: response_id,
                    generation: response_generation,
                    effect,
                }) if response_id == request_id && response_generation == generation => {
                    effect
                        .validate()
                        .map_err(SupervisorError::UnexpectedMessage)?;
                    return Ok(effect);
                }
                Some(Message::HostRequest {
                    request_id: service_request_id,
                    parent_request_id,
                    generation: service_generation,
                    request,
                }) if parent_request_id == request_id && service_generation == generation => {
                    self.handle_host_request(
                        service_request_id,
                        parent_request_id,
                        service_generation,
                        request,
                        &mut interruption,
                    )?;
                }
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) => {
                    if response_id.as_deref() == Some(&request_id) && code == "cancelled" {
                        return Err(SupervisorError::Cancelled("action"));
                    }
                    return Err(extension_reported_error(
                        "action",
                        &request_id,
                        response_id.as_deref(),
                        &code,
                        &message,
                    ));
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
        let mut cancellation_sent = false;
        loop {
            if should_cancel() && !cancellation_sent {
                self.send(&Message::Cancel {
                    request_id: request_id.clone(),
                    generation,
                })?;
                cancellation_sent = true;
            }
            let message = match self.poll_receive(Duration::from_millis(25))? {
                Some(message) => message,
                None => continue,
            };
            match message {
                Some(Message::Snapshot {
                    request_id: response_id,
                    generation: response_generation,
                    complete,
                    entries,
                }) if response_id == request_id && response_generation == generation => {
                    for entry in &entries {
                        nanika_protocol::validate_actions(&entry.actions)
                            .map_err(SupervisorError::UnexpectedMessage)?;
                        if !entry
                            .actions
                            .iter()
                            .any(|action| action.id == entry.action_id)
                        {
                            return Err(SupervisorError::UnexpectedMessage(
                                "candidate default action is not declared".to_owned(),
                            ));
                        }
                    }
                    if !cancellation_sent {
                        publish(entries)?;
                    }
                    if complete {
                        return Ok(!cancellation_sent);
                    }
                }
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) => {
                    if cancellation_sent && response_id.as_deref() == Some(&request_id) {
                        if code != "cancelled" {
                            tracing::warn!(%request_id, %code, %message, "superseded query failed");
                        }
                        return Ok(false);
                    }
                    return Err(extension_reported_error(
                        "query",
                        &request_id,
                        response_id.as_deref(),
                        &code,
                        &message,
                    ));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
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
        mut should_cancel: impl FnMut() -> bool,
    ) -> Result<
        (
            u64,
            nanika_protocol::NavigationEffect,
            Option<nanika_protocol::View>,
        ),
        SupervisorError,
    > {
        self.ensure_initialized()?;
        let request_id = request_id.into();
        let view_id = view_id.into();
        self.send(&Message::ViewEvent {
            request_id: request_id.clone(),
            generation,
            view_id: view_id.clone(),
            revision,
            event,
        })?;
        loop {
            if should_cancel() {
                self.send(&Message::Cancel {
                    request_id: request_id.clone(),
                    generation,
                })?;
                return Err(SupervisorError::Cancelled("view event"));
            }
            let message = match self.poll_receive(Duration::from_millis(25))? {
                Some(message) => message,
                None => continue,
            };
            match message {
                Some(Message::ViewUpdated {
                    request_id: response_id,
                    generation: response_generation,
                    view_id: response_view_id,
                    revision: response_revision,
                    effect,
                    view,
                }) if response_id == request_id
                    && response_generation == generation
                    && response_view_id == view_id =>
                {
                    effect
                        .validate()
                        .map_err(SupervisorError::UnexpectedMessage)?;
                    if let Some(view) = &view {
                        if response_revision <= revision {
                            return Err(SupervisorError::UnexpectedMessage(
                                "extension view revision did not advance".to_owned(),
                            ));
                        }
                        view.validate()
                            .map_err(SupervisorError::UnexpectedMessage)?;
                    } else if response_revision != revision {
                        return Err(SupervisorError::UnexpectedMessage(
                            "extension view revision changed without a replacement view".to_owned(),
                        ));
                    }
                    return Ok((response_revision, effect, view));
                }
                Some(Message::HostRequest {
                    request_id: service_request_id,
                    parent_request_id,
                    generation: service_generation,
                    request,
                }) if parent_request_id == request_id && service_generation == generation => {
                    let mut interruption = || {
                        if should_cancel() {
                            ExtensionInterruption::Cancel
                        } else {
                            ExtensionInterruption::None
                        }
                    };
                    self.handle_host_request(
                        service_request_id,
                        parent_request_id,
                        service_generation,
                        request,
                        &mut interruption,
                    )?;
                }
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) => {
                    return Err(extension_reported_error(
                        "view event",
                        &request_id,
                        response_id.as_deref(),
                        &code,
                        &message,
                    ));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
    }

    pub(crate) fn close_view(
        &mut self,
        request_id: impl Into<String>,
        view_id: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.ensure_initialized()?;
        let request_id = request_id.into();
        let view_id = view_id.into();
        self.send(&Message::ViewClose {
            request_id: request_id.clone(),
            view_id: view_id.clone(),
        })?;
        loop {
            match self.receive()? {
                Some(Message::ViewClosed {
                    request_id: response_id,
                    view_id: response_view_id,
                }) if response_id == request_id && response_view_id == view_id => return Ok(()),
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) => {
                    return Err(extension_reported_error(
                        "view close",
                        &request_id,
                        response_id.as_deref(),
                        &code,
                        &message,
                    ));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    pub fn ensure_running(&mut self) -> Result<(), SupervisorError> {
        let Some(status) = self.child.try_wait()? else {
            return Ok(());
        };
        let stderr = self.stderr_tail();
        let detail = if stderr.trim().is_empty() {
            format!("extension process exited with {status}")
        } else {
            format!("extension process exited with {status}; stderr: {stderr}")
        };
        Err(SupervisorError::UnexpectedMessage(detail))
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
        let child_exited = match self.child.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(error) => {
                first_error = Some(error);
                false
            }
        };
        if let Err(error) = self.process_tree.terminate(self.child.id())
            && first_error.is_none()
        {
            first_error = Some(error);
        }
        if !child_exited
            && let Err(error) = self.child.kill()
            && error.kind() != io::ErrorKind::InvalidInput
            && first_error.is_none()
        {
            first_error = Some(error);
        }
        if !child_exited
            && let Err(error) = self.child.wait()
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
        loop {
            match self.receive()? {
                Some(Message::ShutdownAck {
                    request_id: response_id,
                }) if response_id == request_id => break,
                Some(_) => continue,
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
        self.child.wait()?;
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

    fn receive_configuration_applied(&mut self, request_id: String) -> Result<(), SupervisorError> {
        loop {
            match self.receive()? {
                Some(Message::ConfigurationApplied {
                    request_id: response_id,
                }) if response_id == request_id => return Ok(()),
                Some(Message::Error {
                    request_id: response_id,
                    code,
                    message,
                }) => {
                    return Err(extension_reported_error(
                        "configuration",
                        &request_id,
                        response_id.as_deref(),
                        &code,
                        &message,
                    ));
                }
                Some(_) => {}
                None => return Err(SupervisorError::ChannelClosed),
            }
        }
    }

    fn handle_host_request(
        &mut self,
        request_id: String,
        parent_request_id: String,
        generation: u64,
        request: nanika_protocol::HostServiceRequest,
        interruption: &mut impl FnMut() -> ExtensionInterruption,
    ) -> Result<(), SupervisorError> {
        if matches!(interruption(), ExtensionInterruption::Cancel) {
            return self.send(&Message::Error {
                request_id: Some(request_id),
                code: "cancelled".to_owned(),
                message: "action was cancelled before host service submission".to_owned(),
            });
        }
        let receiver = self
            .extension_id
            .as_deref()
            .zip(self.host_services.as_deref())
            .ok_or_else(|| "host services are unavailable".to_owned())
            .and_then(|(extension_id, services)| services.submit(extension_id, request));
        let result = match receiver {
            Ok(receiver) => loop {
                match interruption() {
                    ExtensionInterruption::None => {}
                    // Accepted side effects cannot be retracted; deliver their outcome before cancellation.
                    ExtensionInterruption::Cancel => {}
                    ExtensionInterruption::Terminate => {
                        self.terminate()?;
                        return Err(SupervisorError::Cancelled("host service"));
                    }
                }
                match receiver.recv_timeout(Duration::from_millis(25)) {
                    Ok(result) => break result,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => {
                        break Err("host service closed before replying".to_owned());
                    }
                }
            },
            Err(error) => Err(error),
        };
        match result {
            Ok(response) => self.send(&Message::HostResponse {
                request_id,
                parent_request_id,
                generation,
                response,
            }),
            Err(message) => self.send(&Message::Error {
                request_id: Some(request_id),
                code: "host_service_failed".to_owned(),
                message,
            }),
        }
    }
}

impl Drop for ExtensionProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

fn drain_stderr(
    mut stderr: impl Read,
    output: &Arc<Mutex<VecDeque<u8>>>,
    byte_limit: usize,
    source: &str,
) {
    let mut chunk = [0; 4096];
    loop {
        let read = match stderr.read(&mut chunk) {
            Ok(0) => break,
            Err(error) => {
                tracing::error!(extension_process = source, %error, "could not read extension stderr");
                break;
            }
            Ok(read) => read,
        };
        tracing::info!(
            extension_process = source,
            message = %String::from_utf8_lossy(&chunk[..read]),
            "extension stderr"
        );
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

fn extension_reported_error(
    operation: &str,
    expected_request_id: &str,
    response_request_id: Option<&str>,
    code: &str,
    message: &str,
) -> SupervisorError {
    let correlation = match response_request_id {
        Some(response_request_id) if response_request_id == expected_request_id => String::new(),
        Some(response_request_id) => format!(
            "uncorrelated error response for request {response_request_id} while waiting for {expected_request_id}: "
        ),
        None => format!(
            "uncorrelated error response without a request id while waiting for {expected_request_id}: "
        ),
    };
    SupervisorError::UnexpectedMessage(format!(
        "{correlation}extension {operation} failed with {code}: {message}"
    ))
}
