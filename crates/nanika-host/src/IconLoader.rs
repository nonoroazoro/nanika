use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::thread::JoinHandle;

use crate::{IconIdentity, IconLoadResult, IconLoaderCommand};

const ICON_COMMAND_CAPACITY: usize = 16;
const MAX_ICON_FILE_BYTES: u64 = 1024 * 1024;
const MAX_ICON_DIMENSION: u32 = 128;

/// Asynchronously decodes bounded extension icon cache entries.
pub(crate) struct IconLoader {
    commands: SyncSender<IconLoaderCommand>,
    results: Receiver<IconLoadResult>,
    thread: Option<JoinHandle<()>>,
}

impl IconLoader {
    pub(crate) fn spawn(
        cache_root: impl Into<PathBuf>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let cache_root = cache_root.into();
        let (commands, receiver) = mpsc::sync_channel(ICON_COMMAND_CAPACITY);
        let (result_sender, results) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("nanika-icon-loader".to_owned())
            .spawn(move || run_loader(&cache_root, receiver, result_sender, wake))?;
        Ok(Self {
            commands,
            results,
            thread: Some(thread),
        })
    }

    pub(crate) fn request(&self, identity: IconIdentity) -> Result<(), String> {
        self.commands
            .try_send(IconLoaderCommand::Load(identity))
            .map_err(|error| match error {
                TrySendError::Full(_) => "icon loader queue is full".to_owned(),
                TrySendError::Disconnected(_) => "icon loader is closed".to_owned(),
            })
    }

    pub(crate) fn take_results(&self) -> Vec<IconLoadResult> {
        self.results.try_iter().collect()
    }
}

impl Drop for IconLoader {
    fn drop(&mut self) {
        let _ = self.commands.send(IconLoaderCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_loader(
    cache_root: &Path,
    commands: Receiver<IconLoaderCommand>,
    results: Sender<IconLoadResult>,
    wake: Arc<dyn Fn() + Send + Sync>,
) {
    while let Ok(command) = commands.recv() {
        let IconLoaderCommand::Load(identity) = command else {
            break;
        };
        let image = load_icon(cache_root, &identity);
        if results.send(IconLoadResult { identity, image }).is_err() {
            break;
        }
        wake();
    }
}

fn load_icon(cache_root: &Path, identity: &IconIdentity) -> Result<egui::ColorImage, String> {
    if !nanika_storage::is_valid_extension_id(identity.extension_id())
        || nanika_protocol::IconReference::new(identity.key()).is_err()
    {
        return Err("icon reference is invalid".to_owned());
    }
    let path = cache_root
        .join("icons")
        .join(identity.extension_id())
        .join(identity.key())
        .join("128.png");
    let file = File::open(&path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if metadata.len() > MAX_ICON_FILE_BYTES {
        return Err("icon file exceeds the encoded size limit".to_owned());
    }
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > MAX_ICON_DIMENSION
        || info.height > MAX_ICON_DIMENSION
        || info.color_type != png::ColorType::Rgba
        || info.bit_depth != png::BitDepth::Eight
    {
        return Err("icon PNG format is unsupported".to_owned());
    }
    let width = usize::try_from(info.width).map_err(|error| error.to_string())?;
    let height = usize::try_from(info.height).map_err(|error| error.to_string())?;
    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| "icon PNG output size is unavailable".to_owned())?;
    let mut pixels = vec![0_u8; buffer_size];
    let output = reader
        .next_frame(&mut pixels)
        .map_err(|error| error.to_string())?;
    let pixels = &pixels[..output.buffer_size()];
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width, height],
        pixels,
    ))
}
