use std::fmt::Write;
use std::path::Path;

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat};
use nanika_protocol::{ClipboardContent, MAX_PNG_ENCODED_BYTES, png_dimensions_within_limits};
use sha2::{Digest, Sha256};

use crate::ClipboardEntry;

const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_FILES: usize = 256;
pub(crate) fn capture(
    context: &ClipboardContext,
    payload_root: &Path,
    captured_at: u64
) -> Result<Option<ClipboardEntry>, String> {
    if context.has(ContentFormat::Files) {
        let paths = context.get_files().map_err(|error| error.to_string())?;
        if !paths.is_empty() {
            let encoded = serde_json::to_vec(&paths).map_err(|error| error.to_string())?;
            if !files_within_limits(&paths, encoded.len()) {
                return Err(format!(
                    "clipboard file list exceeds the supported limit: {} files, {} encoded bytes",
                    paths.len(),
                    encoded.len()
                ));
            }
            let hash = stable_hash("files", &encoded);
            return Ok(Some(ClipboardEntry {
                entry_id: hash,
                image_dimensions: None,
                content: ClipboardContent::Files { paths },
                byte_size: encoded.len() as u64,
                captured_at
            }));
        }
    }
    if context.has(ContentFormat::Text) {
        let value = context.get_text().map_err(|error| error.to_string())?;
        if !text_within_limits(&value) {
            return Err(format!(
                "clipboard text exceeds the supported limit: {} bytes",
                value.len()
            ));
        }
        if !value.trim().is_empty() {
            let hash = stable_hash("text", value.as_bytes());
            return Ok(Some(ClipboardEntry {
                entry_id: hash,
                image_dimensions: None,
                byte_size: value.len() as u64,
                content: ClipboardContent::Text { value },
                captured_at
            }));
        }
    }
    if context.has(ContentFormat::Image) {
        let image = context.get_image().map_err(|error| error.to_string())?;
        let (width, height) = image.get_size();
        if !png_dimensions_within_limits(width, height) {
            return Err(format!(
                "clipboard image exceeds the supported dimensions: {width} x {height}"
            ));
        }
        let encoded = image.to_png().map_err(|error| error.to_string())?;
        let bytes = encoded.get_bytes();
        if bytes.len() > MAX_PNG_ENCODED_BYTES {
            return Err(format!(
                "clipboard image exceeds the supported encoded size: {} bytes",
                bytes.len()
            ));
        }
        let hash = stable_hash("image", bytes);
        std::fs::create_dir_all(payload_root).map_err(|error| error.to_string())?;
        let path = payload_root.join(format!("{hash}.png"));
        if !path.is_file() {
            write_atomic(&path, bytes)?;
        }
        return Ok(Some(ClipboardEntry {
            entry_id: hash,
            image_dimensions: Some(crate::ClipboardImageDimensions { width, height }),
            content: ClipboardContent::PngFile {
                path: path.to_string_lossy().into_owned()
            },
            byte_size: bytes.len() as u64,
            captured_at
        }));
    }
    Ok(None)
}

pub(crate) fn files_within_limits(paths: &[String], encoded_bytes: usize) -> bool {
    paths.len() <= MAX_FILES && encoded_bytes <= MAX_TEXT_BYTES
}

pub(crate) fn text_within_limits(value: &str) -> bool {
    value.len() <= MAX_TEXT_BYTES
}

pub(crate) fn stable_hash(kind: &str, payload: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    digest.update([0]);
    digest.update(payload);
    let digest = digest.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    match std::fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_error) if path.is_file() => {
            let _ = std::fs::remove_file(temporary);
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::remove_file(temporary);
            Err(error.to_string())
        }
    }
}
