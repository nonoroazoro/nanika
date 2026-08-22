use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;

use nanika_config::ConfigStore;

use crate::{HostConfig, HostConfigCommand};

const COMMAND_CAPACITY: usize = 4;

/// Bounded owner for host JSONC mutations.
pub(crate) struct HostConfigService {
    commands: SyncSender<HostConfigCommand>,
    thread: Option<JoinHandle<()>>,
}

impl HostConfigService {
    pub(crate) fn spawn(store: ConfigStore) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(COMMAND_CAPACITY);
        let thread = std::thread::Builder::new()
            .name("nanika-host-config-owner".to_owned())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        HostConfigCommand::Update {
                            hotkey,
                            reduced_motion,
                            response,
                        } => {
                            let result = store
                                .update::<HostConfig>(
                                    store.config_file(),
                                    [
                                        ("hotkey".to_owned(), serde_json::json!(hotkey)),
                                        (
                                            "reducedMotion".to_owned(),
                                            serde_json::json!(reduced_motion),
                                        ),
                                    ],
                                    HostConfig::validate,
                                )
                                .map_err(|error| error.to_string());
                            let _ = response.send(result);
                        }
                        HostConfigCommand::Shutdown => break,
                    }
                }
            })?;
        Ok(Self {
            commands,
            thread: Some(thread),
        })
    }

    pub(crate) fn update(
        &self,
        hotkey: String,
        reduced_motion: bool,
    ) -> Result<Receiver<Result<HostConfig, String>>, String> {
        let (response, receiver) = mpsc::sync_channel(1);
        self.commands
            .try_send(HostConfigCommand::Update {
                hotkey,
                reduced_motion,
                response,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => "host config queue is full".to_owned(),
                TrySendError::Disconnected(_) => "host config owner has stopped".to_owned(),
            })?;
        Ok(receiver)
    }

    pub(crate) fn shutdown(mut self) {
        let _ = self.commands.send(HostConfigCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for HostConfigService {
    fn drop(&mut self) {
        let _ = self.commands.send(HostConfigCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
