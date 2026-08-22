use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;

use crate::{PlatformError, StartupCommand, StartupStatus};

const COMMAND_CAPACITY: usize = 4;

/// Bounded owner for login startup status and mutations.
pub struct StartupService {
    commands: SyncSender<StartupCommand>,
    thread: Option<JoinHandle<()>>,
}

impl StartupService {
    pub fn spawn(executable: PathBuf) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(COMMAND_CAPACITY);
        let thread = std::thread::Builder::new()
            .name("nanika-startup-owner".to_owned())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        StartupCommand::Query { response } => {
                            let _ = response.send(crate::startup_status(&executable));
                        }
                        StartupCommand::SetEnabled { enabled, response } => {
                            let _ = response.send(crate::set_startup_enabled(&executable, enabled));
                        }
                        StartupCommand::Shutdown => break,
                    }
                }
            })?;
        Ok(Self {
            commands,
            thread: Some(thread),
        })
    }

    pub fn query(&self) -> Result<Receiver<Result<StartupStatus, PlatformError>>, PlatformError> {
        let (response, receiver) = mpsc::sync_channel(1);
        self.submit(StartupCommand::Query { response })?;
        Ok(receiver)
    }

    pub fn set_enabled(
        &self,
        enabled: bool,
    ) -> Result<Receiver<Result<StartupStatus, PlatformError>>, PlatformError> {
        let (response, receiver) = mpsc::sync_channel(1);
        self.submit(StartupCommand::SetEnabled { enabled, response })?;
        Ok(receiver)
    }

    fn submit(&self, command: StartupCommand) -> Result<(), PlatformError> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                TrySendError::Full(_) => PlatformError::QueueFull("startup"),
                TrySendError::Disconnected(_) => PlatformError::EventChannelClosed("startup"),
            })
    }

    pub fn shutdown(mut self) {
        let _ = self.commands.send(StartupCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for StartupService {
    fn drop(&mut self) {
        let _ = self.commands.send(StartupCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
