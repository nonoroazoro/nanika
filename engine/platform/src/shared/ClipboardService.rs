use nanika_protocol::{ClipboardContent, HostServiceResponse};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

use crate::{ClipboardServiceCommand, read_png_resource};
use std::sync::Arc;

/// Bounded native clipboard writer owned by one platform thread.
pub struct ClipboardService {
    commands: SyncSender<ClipboardServiceCommand>,
    budget: Arc<crate::ClipboardWriteBudget>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardService {
    pub fn spawn() -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-service".to_owned())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        ClipboardServiceCommand::Write { prepared, response } => {
                            if response.send(write(prepared)).is_err() {
                                tracing::warn!(
                                    "clipboard requester closed before receiving the result"
                                );
                            }
                        }
                        ClipboardServiceCommand::Shutdown => break,
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            commands,
            budget: Arc::default(),
            thread: Some(thread),
        })
    }

    pub fn prepare(
        &self,
        content: ClipboardContent,
        payload_root: Option<&std::path::Path>,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<crate::PreparedClipboardWrite, String> {
        let permit = self.budget.acquire(
            matches!(&content, ClipboardContent::PngFile { .. }),
            cancelled,
        )?;
        let content = match content {
            ClipboardContent::Text { value } => crate::PreparedClipboardContent::Text(value),
            ClipboardContent::Files { paths } => crate::PreparedClipboardContent::Files(paths),
            ClipboardContent::PngFile { path } => {
                let root = payload_root.ok_or("clipboard image payload root is unavailable")?;
                let bytes = read_png_resource(std::path::Path::new(&path), root)
                    .map_err(|error| error.to_string())?;
                crate::validate_png_pixels(&bytes, cancelled).map_err(|error| error.to_string())?;
                crate::PreparedClipboardContent::Png(bytes)
            }
        };
        Ok(crate::PreparedClipboardWrite { content, permit })
    }

    pub fn submit(
        &self,
        prepared: crate::PreparedClipboardWrite,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardServiceCommand::Write { prepared, response })
            .map_err(|_| "clipboard service is closed".to_owned())?;
        Ok(result)
    }

    fn stop(&mut self) {
        if self
            .commands
            .send(ClipboardServiceCommand::Shutdown)
            .is_err()
        {
            tracing::error!("clipboard service closed before shutdown was requested");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("clipboard service thread panicked during shutdown");
        }
    }
}

impl Drop for ClipboardService {
    fn drop(&mut self) {
        self.stop();
    }
}

fn write(prepared: crate::PreparedClipboardWrite) -> Result<HostServiceResponse, String> {
    let crate::PreparedClipboardWrite { content, permit } = prepared;
    let result = crate::adapter::clipboard::write(content)
        .map(|revision| HostServiceResponse::ClipboardWritten { revision });
    drop(permit);
    result
}
