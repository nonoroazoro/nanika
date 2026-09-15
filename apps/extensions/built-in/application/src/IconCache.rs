use std::fs;
use std::path::{Path, PathBuf};

use crate::normalization::{path_key, stable_hash, timestamp_nanos};
use crate::platform;
use crate::{ApplicationEntry, ApplicationError, DiscoveryState};

const ICON_SIZES: [u32; 3] = [32, 64, 128];
const FALLBACK_KEY: &str = "application-fallback-v1";
const ICON_RENDER_VERSION: &str = "normalized-v5";

/// Machine-local icon cache with deterministic content keys.
pub struct IconCache {
    root: PathBuf,
}

impl IconCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub const fn fallback_key() -> &'static str {
        FALLBACK_KEY
    }

    pub(crate) fn key(&self, entry: &ApplicationEntry) -> Result<String, ApplicationError> {
        let Some(source) = entry.icon_source.as_deref() else {
            return Ok(FALLBACK_KEY.to_owned());
        };
        let metadata = source.metadata()?;
        icon_key(entry, source, &metadata)
    }

    pub(crate) fn key_with_state(
        &self,
        entry: &ApplicationEntry,
        state: &mut DiscoveryState,
    ) -> Result<String, ApplicationError> {
        let Some(source) = entry.icon_source.as_deref() else {
            return Ok(FALLBACK_KEY.to_owned());
        };
        let metadata = state.metadata(source)?;
        icon_key(entry, source, metadata)
    }

    pub fn prepare(&self, entry: &mut ApplicationEntry) -> Result<(), ApplicationError> {
        let key = if entry.icon_key.is_empty() {
            self.key(entry)?
        } else {
            entry.icon_key.clone()
        };
        if key == FALLBACK_KEY {
            self.ensure_fallback()?;
            entry.icon_key = key;
            return Ok(());
        }
        let Some(source) = entry.icon_source.as_deref() else {
            return Ok(());
        };
        let directory = self.root.join(&key);
        fs::create_dir_all(&directory)?;
        let fallback_marker = directory.join("fallback.marker");
        let retry_fallback = fallback_marker.is_file();
        if retry_fallback {
            for size in ICON_SIZES {
                let target = directory.join(format!("{size}.png"));
                if let Err(error) = fs::remove_file(&target)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(error.into());
                }
            }
        }
        if retry_fallback
            || ICON_SIZES
                .iter()
                .any(|size| !directory.join(format!("{size}.png")).is_file())
        {
            fs::write(&fallback_marker, [])?;
        }
        for size in ICON_SIZES {
            let target = directory.join(format!("{size}.png"));
            if !target.is_file()
                && let Err(error) = platform::extract_icon(source, entry.icon_index, size, &target)
            {
                self.copy_fallback_to(&directory)?;
                return Err(error);
            }
        }
        if let Err(error) = fs::remove_file(fallback_marker)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        entry.icon_key = key;
        Ok(())
    }

    pub(crate) fn use_available_icons(
        &self,
        entries: &mut [ApplicationEntry],
    ) -> Result<(), ApplicationError> {
        self.ensure_fallback()?;
        for entry in entries {
            if !self.is_ready(&entry.icon_key) {
                entry.icon_key = FALLBACK_KEY.to_owned();
            }
        }
        Ok(())
    }

    fn ensure_fallback(&self) -> Result<(), ApplicationError> {
        let directory = self.root.join(FALLBACK_KEY);
        fs::create_dir_all(&directory)?;
        for size in ICON_SIZES {
            let target = directory.join(format!("{size}.png"));
            if !target.is_file() {
                write_fallback_icon(&target, size)?;
            }
        }
        Ok(())
    }

    fn is_ready(&self, key: &str) -> bool {
        if key == FALLBACK_KEY {
            return true;
        }
        if nanika_protocol::IconReference::new(key).is_err() {
            return false;
        }
        let directory = self.root.join(key);
        !directory.join("fallback.marker").is_file()
            && ICON_SIZES
                .iter()
                .all(|size| directory.join(format!("{size}.png")).is_file())
    }

    fn copy_fallback_to(&self, target: &Path) -> Result<(), ApplicationError> {
        self.ensure_fallback()?;
        let source = self.root.join(FALLBACK_KEY);
        for size in ICON_SIZES {
            let target = target.join(format!("{size}.png"));
            fs::copy(source.join(format!("{size}.png")), target)?;
        }
        Ok(())
    }
}

fn icon_key(
    entry: &ApplicationEntry,
    source: &Path,
    metadata: &std::fs::Metadata,
) -> Result<String, ApplicationError> {
    let modified = timestamp_nanos(metadata.modified()?);
    Ok(key_from_stamp(
        source,
        entry.icon_index,
        metadata.len(),
        modified,
    ))
}

pub(crate) fn key_from_stamp(
    source: &Path,
    icon_index: i32,
    length: u64,
    modified: i128,
) -> String {
    stable_hash(&[
        ICON_RENDER_VERSION,
        &path_key(source),
        &icon_index.to_string(),
        &length.to_string(),
        &modified.to_string(),
    ])
}

fn write_fallback_icon(path: &Path, size: u32) -> Result<(), ApplicationError> {
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let index = ((y * size + x) * 4) as usize;
            let inset = size / 6;
            let inside = x >= inset && y >= inset && x < size - inset && y < size - inset;
            let color = if inside {
                [78, 91, 126, 255]
            } else {
                [0, 0, 0, 0]
            };
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
    write_png(path, size, size, &pixels)
}

pub(crate) fn write_png(
    path: &Path,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<(), ApplicationError> {
    let temporary = path.with_extension("png.tmp");
    let file = fs::File::create(&temporary)?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
    writer
        .write_image_data(pixels)
        .map_err(std::io::Error::other)?;
    writer.finish().map_err(std::io::Error::other)?;
    fs::rename(temporary, path)?;
    Ok(())
}
