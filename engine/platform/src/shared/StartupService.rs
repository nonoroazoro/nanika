use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use crate::{PlatformError, StartupCommand, StartupStatus};

/// Owner for login startup status and mutations. Submission applies backpressure.
pub struct StartupService {
    commands: Sender<StartupCommand>,
    thread: Option<JoinHandle<()>>,
}

impl StartupService {
    pub fn spawn(executable: PathBuf) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::channel();
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
            .send(command)
            .map_err(|_| PlatformError::EventChannelClosed("startup"))
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        if self.commands.send(StartupCommand::Shutdown).is_err() {
            tracing::error!("startup service closed before shutdown was requested");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("startup service owner thread panicked");
        }
    }
}

impl Drop for StartupService {
    fn drop(&mut self) {
        self.stop();
    }
}
