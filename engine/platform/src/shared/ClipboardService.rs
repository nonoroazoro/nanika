#[cfg(target_os = "macos")]
use clipboard_rs::RustImageData;
#[cfg(target_os = "macos")]
use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, Result as ClipboardResult};
use nanika_protocol::{ClipboardContent, HostServiceResponse};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

use crate::{ClipboardServiceCommand, read_png_resource};

/// Bounded native clipboard writer owned by one platform thread.
pub struct ClipboardService {
    commands: SyncSender<ClipboardServiceCommand>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardService {
    pub fn spawn() -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let (ready, initialized) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-service".to_owned())
            .spawn(move || {
                let context = match ClipboardContext::new() {
                    Ok(context) => context,
                    Err(error) => {
                        let _ = ready.send(Err(error.to_string()));
                        return;
                    }
                };
                if ready.send(Ok(())).is_err() {
                    return;
                }
                while let Ok(command) = receiver.recv() {
                    match command {
                        ClipboardServiceCommand::Write {
                            content,
                            payload_root,
                            response,
                        } => {
                            if response
                                .send(write(&context, content, &payload_root))
                                .is_err()
                            {
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
        initialized
            .recv()
            .map_err(|_| "clipboard service closed during initialization".to_owned())??;
        Ok(Self {
            commands,
            thread: Some(thread),
        })
    }

    pub fn submit(
        &self,
        content: ClipboardContent,
        payload_root: Option<std::path::PathBuf>,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardServiceCommand::Write {
                content,
                payload_root,
                response,
            })
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

fn write(
    context: &ClipboardContext,
    content: ClipboardContent,
    payload_root: &Option<std::path::PathBuf>,
) -> Result<HostServiceResponse, String> {
    match content {
        ClipboardContent::Text { value } => context.set_text(value),
        ClipboardContent::Files { paths } => context.set_files(paths),
        ClipboardContent::PngFile { path } => {
            let payload_root = payload_root
                .as_deref()
                .ok_or_else(|| "clipboard image payload root is unavailable".to_owned())?;
            let bytes = read_png_resource(std::path::Path::new(&path), payload_root)
                .map_err(|error| error.to_string())?;
            write_png(context, bytes)
        }
    }
    .map_err(|error| error.to_string())?;
    Ok(HostServiceResponse::ClipboardWritten {
        revision: crate::clipboard_revision(),
    })
}

fn write_png(context: &ClipboardContext, bytes: Vec<u8>) -> ClipboardResult<()> {
    #[cfg(target_os = "windows")]
    {
        // Preserve the captured PNG bytes. Decoding, re-encoding PNG, and then
        // converting to a bitmap made large-image copy operations unnecessarily slow.
        context.set_buffer("PNG", bytes)
    }
    #[cfg(target_os = "macos")]
    {
        let image = RustImageData::from_bytes(&bytes)?;
        context.set_image(image)
    }
}
