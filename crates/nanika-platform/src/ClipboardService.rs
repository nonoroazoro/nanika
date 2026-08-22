use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::JoinHandle;
use std::time::Instant;
use std::{fs::File, io::Read};

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, RustImageData};
use nanika_protocol::{ClipboardContent, HostServiceResponse};

use crate::ClipboardServiceCommand;

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
                            deadline,
                            response,
                        } => {
                            let _ =
                                response.send(write(&context, content, &payload_root, deadline));
                        }
                        ClipboardServiceCommand::Shutdown => break,
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        initialized
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| "clipboard service initialization timed out".to_owned())??;
        Ok(Self {
            commands,
            thread: Some(thread),
        })
    }

    pub fn submit(
        &self,
        content: ClipboardContent,
        payload_root: Option<std::path::PathBuf>,
        deadline: Instant,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .try_send(ClipboardServiceCommand::Write {
                content,
                payload_root,
                deadline,
                response,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => "clipboard service queue is full".to_owned(),
                TrySendError::Disconnected(_) => "clipboard service is closed".to_owned(),
            })?;
        Ok(result)
    }

    fn stop(&mut self) {
        let _ = self.commands.try_send(ClipboardServiceCommand::Shutdown);
        self.thread.take();
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
    deadline: Instant,
) -> Result<HostServiceResponse, String> {
    ensure_before_deadline(deadline)?;
    match content {
        ClipboardContent::Text { value } => context.set_text(value),
        ClipboardContent::Files { paths } => context.set_files(paths),
        ClipboardContent::PngFile { path } => {
            let payload_root = payload_root
                .as_deref()
                .ok_or_else(|| "clipboard image payload root is unavailable".to_owned())?;
            let bytes = read_validated_png(&path, payload_root)?;
            let image = RustImageData::from_bytes(&bytes).map_err(|error| error.to_string())?;
            ensure_before_deadline(deadline)?;
            context.set_image(image)
        }
    }
    .map(|()| HostServiceResponse::ClipboardWritten)
    .map_err(|error| error.to_string())
}

pub(crate) fn read_validated_png(
    path: &str,
    payload_root: &std::path::Path,
) -> Result<Vec<u8>, String> {
    const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;
    const MAX_DIMENSION: u32 = 8_192;
    const MAX_PIXELS: u64 = 16_777_216;

    std::fs::create_dir_all(payload_root).map_err(|error| error.to_string())?;
    let payload_root = payload_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !path.starts_with(&payload_root) || !path.is_file() {
        return Err("clipboard image is outside the extension payload root".to_owned());
    }
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if metadata.len() > MAX_ENCODED_BYTES as u64 {
        return Err("clipboard image exceeds the encoded size limit".to_owned());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take((MAX_ENCODED_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_ENCODED_BYTES {
        return Err("clipboard image exceeds the encoded size limit".to_owned());
    }
    let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    let reader = decoder.read_info().map_err(|error| error.to_string())?;
    let info = reader.info();
    let pixels = u64::from(info.width).saturating_mul(u64::from(info.height));
    if info.width > MAX_DIMENSION || info.height > MAX_DIMENSION || pixels > MAX_PIXELS {
        return Err("clipboard image exceeds the dimension limit".to_owned());
    }
    Ok(bytes)
}

pub(crate) fn ensure_before_deadline(deadline: Instant) -> Result<(), String> {
    if Instant::now() >= deadline {
        Err("clipboard request expired before execution".to_owned())
    } else {
        Ok(())
    }
}
