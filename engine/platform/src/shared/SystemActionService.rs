use nanika_protocol::{HostServiceResponse, SystemAction};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;

/// Lazy owner of blocking OS operations. Dropping it drains all admitted work.
pub struct SystemActionService {
    _commands: Option<SyncSender<SystemAction>>,
    _thread: Option<JoinHandle<()>>
}

impl SystemActionService {
    pub fn spawn() -> std::io::Result<Self> {
        Self::_spawn(crate::adapter::system_action::executor()?)
    }

    pub fn submit(
        &self,
        action: SystemAction
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, receiver) = mpsc::sync_channel(1);
        self._commands
            .as_ref()
            .ok_or("System service is stopped.")?
            .try_send(action)
            .map_err(|error| {
                match error {
                    TrySendError::Full(_) => "System service is busy. The action was not accepted.",
                    TrySendError::Disconnected(_) => {
                        "System service is unavailable. The action was not accepted."
                    }
                }
                .to_owned()
            })?;
        // Receipt means host ownership, never successful OS execution.
        let _ = response.send(Ok(HostServiceResponse::SystemActionSubmitted));
        Ok(receiver)
    }

    fn _spawn(
        execute: impl Fn(SystemAction) -> Result<(), String> + Send + 'static
    ) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel::<SystemAction>(16);
        let thread = std::thread::Builder::new()
            .name("nanika-system-owner".to_owned())
            .spawn(move || {
                while let Ok(action) = receiver.recv() {
                    // Native calls can block, but they never hold the launcher open.
                    if let Err(error) = execute(action) {
                        tracing::warn!(?action, %error, "system command dispatch failed");
                    }
                }
            })?;
        Ok(Self {
            _commands: Some(commands),
            _thread: Some(thread)
        })
    }
}

impl Drop for SystemActionService {
    fn drop(&mut self) {
        self._commands.take();
        if let Some(thread) = self._thread.take()
            && thread.join().is_err()
        {
            tracing::error!("system service owner thread panicked");
        }
    }
}

#[cfg(test)]
#[path = "../../tests/shared/SystemActionService.rs"]
mod tests;
