use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nanika_protocol::{HostServiceResponse, LaunchDescriptor};

use crate::LauncherCommand;
#[cfg(not(target_os = "macos"))]
use crate::process_launch::process_launch;

/// Single owner for processes requested through the host service boundary.
pub struct ProcessLauncher {
    commands: SyncSender<LauncherCommand>,
    thread: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    notifier: i32,
}

impl ProcessLauncher {
    pub fn spawn() -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(16);
        #[cfg(target_os = "macos")]
        let notifier = crate::process_launcher_macos::create_queue()?;
        #[cfg(target_os = "macos")]
        let owner_notifier = notifier;
        #[cfg(not(target_os = "macos"))]
        let owner_notifier = ();
        let shutdown = Arc::new(AtomicBool::new(false));
        let owner_shutdown = Arc::clone(&shutdown);
        let thread = std::thread::Builder::new()
            .name("nanika-process-launcher".to_owned())
            .spawn(move || run_owner(receiver, owner_notifier, owner_shutdown));
        #[cfg(target_os = "macos")]
        let thread =
            thread.inspect_err(|_| crate::process_launcher_macos::close_queue(notifier))?;
        #[cfg(not(target_os = "macos"))]
        let thread = thread?;
        Ok(Self {
            commands,
            thread: Some(thread),
            shutdown,
            #[cfg(target_os = "macos")]
            notifier,
        })
    }

    pub fn submit(
        &self,
        descriptor: LaunchDescriptor,
        deadline: Instant,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .try_send(LauncherCommand::Launch {
                descriptor,
                deadline,
                response,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => "process launcher queue is full".to_owned(),
                TrySendError::Disconnected(_) => "process launcher is closed".to_owned(),
            })?;
        self.wake()?;
        Ok(result)
    }

    pub fn launch(&self, descriptor: LaunchDescriptor) -> Result<(), String> {
        self.submit(descriptor, Instant::now() + Duration::from_secs(5))?
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "process launcher did not reply before the deadline".to_owned())??;
        Ok(())
    }

    fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let _ = self.commands.try_send(LauncherCommand::Shutdown);
        let _ = self.wake();
        self.thread.take();
    }

    #[cfg(target_os = "macos")]
    fn wake(&self) -> Result<(), String> {
        crate::process_launcher_macos::wake(self.notifier).map_err(|error| error.to_string())
    }

    #[cfg(not(target_os = "macos"))]
    fn wake(&self) -> Result<(), String> {
        Ok(())
    }
}

impl Drop for ProcessLauncher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_os = "macos")]
fn run_owner(receiver: Receiver<LauncherCommand>, notifier: i32, shutdown: Arc<AtomicBool>) {
    crate::process_launcher_macos::run(receiver, notifier, shutdown);
}

#[cfg(not(target_os = "macos"))]
fn run_owner(receiver: Receiver<LauncherCommand>, _notifier: (), shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Acquire) {
        let Ok(command) = receiver.recv() else {
            break;
        };
        match command {
            LauncherCommand::Launch {
                descriptor,
                deadline,
                response,
            } => {
                let result = if Instant::now() >= deadline {
                    Err("process launch request expired before execution".to_owned())
                } else {
                    process_launch(&descriptor)
                        .map(|child| {
                            drop(child);
                            HostServiceResponse::Launched
                        })
                        .map_err(|error| error.to_string())
                };
                let _ = response.send(result);
            }
            LauncherCommand::Shutdown => break,
        }
    }
}
