use nanika_protocol::IconReference;
use std::fs;
use std::path::{Path, PathBuf};

use crate::normalization::{path_key, stable_hash};
use crate::platform;
use crate::{ApplicationEntry, ApplicationError, DiscoveryState};

const ICON_SIZES: [u32; 3] = [32, 64, 128];
const FALLBACK_KEY: &str = "application-fallback-v1";
const ICON_RENDER_VERSION: &str = "alpha-cropped-v1";

/// Machine-local icon cache with deterministic content keys.
pub struct IconCache {
    root: PathBuf
}

impl IconCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub const fn fallback_key() -> &'static str {
        FALLBACK_KEY
    }

    pub(crate) fn key(
        entry: &ApplicationEntry,
        state: &mut DiscoveryState
    ) -> Result<String, ApplicationError> {
        let Some(source) = entry.icon_source.as_ref() else {
            return Ok(FALLBACK_KEY.to_owned());
        };
        match source {
            crate::ApplicationIconSource::File { path, index } => {
                platform::icon_cache_key(path, *index, state)
            }
            crate::ApplicationIconSource::WindowsApplication {
                app_user_model_id,
                package_full_name
            } => Ok(stable_hash(&[
                ICON_RENDER_VERSION,
                "windows-app",
                app_user_model_id,
                package_full_name
            ]))
        }
    }

    /// Prepare files without changing discovery metadata or published presentation.
    pub fn prepare(&self, entry: &ApplicationEntry) -> Result<IconReference, ApplicationError> {
        // Persisted entries can reach the icon worker before discovery revalidates them.
        // Reject path-shaped keys before any cache directory creation or repair.
        let icon = IconReference::new(&entry.icon_key)
            .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))?;
        if icon.key() == FALLBACK_KEY {
            return self.fallback();
        }
        let source = entry.icon_source.as_ref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "application icon source is missing"
            )
        })?;
        let directory = self.root.join(icon.key());
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
        let missing = ICON_SIZES
            .into_iter()
            .filter(|size| !directory.join(format!("{size}.png")).is_file())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            // An incomplete set stays marked and is never published. The worker owns
            // failure presentation through the single shared fallback icon.
            match source {
                crate::ApplicationIconSource::File { path, index } => {
                    platform::extract_icons(path, *index, &missing, &directory)?
                }
                crate::ApplicationIconSource::WindowsApplication {
                    app_user_model_id, ..
                } => {
                    // One native render supplies the complete resolution set.
                    let native =
                        nanika_platform::windows_application_icon_pixels(app_user_model_id, 256)?;
                    for size in missing {
                        let pixels = nanika_platform::normalize_icon_rgba(&native, 256, 256, size)
                            .ok_or_else(|| {
                                std::io::Error::other("Windows provided an empty application icon")
                            })?;
                        write_png(&directory.join(format!("{size}.png")), size, size, &pixels)?;
                    }
                }
            }
        }
        if let Err(error) = fs::remove_file(fallback_marker)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        Ok(icon)
    }

    pub(crate) fn cached(&self, key: &str) -> Option<IconReference> {
        let icon = IconReference::new(key).ok()?;
        let directory = self.root.join(icon.key());
        (!directory.join("fallback.marker").is_file()
            && ICON_SIZES
                .iter()
                .all(|size| directory.join(format!("{size}.png")).is_file()))
        .then_some(icon)
    }

    pub(crate) fn fallback(&self) -> Result<IconReference, ApplicationError> {
        self._ensure_fallback()?;
        Ok(IconReference::new(FALLBACK_KEY).expect("constant fallback key is valid"))
    }

    fn _ensure_fallback(&self) -> Result<(), ApplicationError> {
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
}

pub(crate) fn key_from_stamp(
    source: &Path,
    icon_index: i32,
    length: u64,
    modified: i128
) -> String {
    stable_hash(&[
        ICON_RENDER_VERSION,
        &path_key(source),
        &icon_index.to_string(),
        &length.to_string(),
        &modified.to_string()
    ])
}

fn write_fallback_icon(path: &Path, size: u32) -> Result<(), ApplicationError> {
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    let stroke = (size / 16).max(1);
    let left = size / 8;
    let right = size - left;
    for y in 0..size {
        for x in 0..size {
            let index = ((y * size + x) * 4) as usize;
            let outline = (left..right).contains(&x)
                && (x < left + stroke || x >= right - stroke || y < stroke || y >= size - stroke);
            let text = (size / 4..size * 3 / 4).contains(&x)
                && ((size * 3 / 8..size * 3 / 8 + stroke).contains(&y)
                    || (size / 2..size / 2 + stroke).contains(&y)
                    || (x < size / 2 && (size * 5 / 8..size * 5 / 8 + stroke).contains(&y)));
            let color = if outline || text {
                [120, 130, 150, 255]
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
    pixels: &[u8]
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
