use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

use nanika_protocol::{HostServiceResponse, LaunchDescriptor};

use crate::LauncherCommand;

/// Single owner for processes requested through the host service boundary.
pub struct ProcessLauncher {
    commands: Option<SyncSender<LauncherCommand>>,
    thread: Option<JoinHandle<()>>,
    #[cfg(target_os = "macos")]
    notifier: i32
}

impl ProcessLauncher {
    pub fn spawn() -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(16);
        #[cfg(target_os = "macos")]
        let notifier = crate::adapter::process_launcher::create_queue()?;
        #[cfg(target_os = "macos")]
        let owner_notifier = notifier;
        #[cfg(windows)]
        let owner_notifier = ();
        let thread = std::thread::Builder::new()
            .name("nanika-process-launcher".to_owned())
            .spawn(move || run_owner(receiver, owner_notifier));
        #[cfg(target_os = "macos")]
        let thread =
            thread.inspect_err(|_| crate::adapter::process_launcher::close_queue(notifier))?;
        #[cfg(windows)]
        let thread = thread?;
        Ok(Self {
            commands: Some(commands),
            thread: Some(thread),
            #[cfg(target_os = "macos")]
            notifier
        })
    }

    pub fn submit(
        &self,
        descriptor: LaunchDescriptor
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or("process launcher is closed")?
            .send(LauncherCommand::Launch {
                descriptor,
                response
            })
            .map_err(|_| "process launcher is closed".to_owned())?;
        if let Err(error) = self.wake() {
            tracing::error!(%error, "could not wake launcher after admission; retain the accepted receipt");
        }
        Ok(result)
    }

    pub fn launch(&self, descriptor: LaunchDescriptor) -> Result<(), String> {
        self.submit(descriptor)?
            .recv()
            .map_err(|_| "process launcher closed without replying".to_owned())??;
        Ok(())
    }

    pub fn reveal(
        &self,
        path: String
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let target = std::path::Path::new(&path);
        if !target.is_absolute() || path.len() > 32768 || path.contains('\0') {
            return Err("invalid reveal path".to_owned());
        }
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .as_ref()
            .ok_or("process launcher is closed")?
            .send(LauncherCommand::Reveal { path, response })
            .map_err(|_| "process launcher is closed".to_owned())?;
        if let Err(error) = self.wake() {
            tracing::error!(%error, "could not wake launcher after admission; retain the accepted receipt");
        }
        Ok(result)
    }

    fn stop(&mut self) {
        // Closing admission wakes/drains the owner without abandoning queued effects.
        self.commands.take();
        if let Err(error) = self.wake() {
            tracing::error!(%error, "failed to wake process launcher for shutdown");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("process launcher thread panicked during shutdown");
        }
    }

    #[cfg(target_os = "macos")]
    fn wake(&self) -> Result<(), String> {
        crate::adapter::process_launcher::wake(self.notifier).map_err(|error| error.to_string())
    }

    #[cfg(windows)]
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
fn run_owner(receiver: Receiver<LauncherCommand>, notifier: i32) {
    crate::adapter::process_launcher::run(receiver, notifier);
}

#[cfg(windows)]
fn run_owner(receiver: Receiver<LauncherCommand>, _notifier: ()) {
    crate::adapter::process_launcher::run(receiver, ());
}
