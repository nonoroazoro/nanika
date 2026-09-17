use std::fs;
use std::path::{Path, PathBuf};

use nanika_protocol::IconReference;
use sha2::{Digest, Sha256};

const RENDER_VERSION: &str = "system-file-icon-v1";

/// Persistent metadata-keyed reuse with immutable, extension-owned PNG artifacts.
pub struct FileIconCache {
    root: PathBuf,
}

impl FileIconCache {
    pub fn new(extension_icon_root: PathBuf) -> Self {
        Self {
            root: extension_icon_root,
        }
    }

    pub fn get(&mut self, path: &Path) -> std::io::Result<IconReference> {
        if let Some(reference) = self.cached(path)? {
            return Ok(reference);
        }
        let reference = self.reference(path)?;
        if variants_exist(&self.root, &reference) {
            return Ok(reference);
        }
        let pixels = crate::file_icon_pixels(path, 0, 512)?;
        let large = crate::normalize_icon_rgba(&pixels, 512, 512, 512)
            .ok_or_else(|| std::io::Error::other("system file icon is empty"))?;
        let png = encode_png(&large, 512)?;
        let small = crate::normalize_icon_rgba(&pixels, 512, 512, 128)
            .ok_or_else(|| std::io::Error::other("system file icon is empty"))?;
        let small_png = encode_png(&small, 128)?;
        let directory = self.root.join(reference.key());
        fs::create_dir_all(&directory)?;
        for (size, bytes) in [(128, small_png), (512, png)] {
            let target = directory.join(format!("{size}.png"));
            if !target.is_file() {
                let temporary = target.with_extension(format!("png.{}.tmp", std::process::id()));
                fs::write(&temporary, bytes)?;
                crate::atomic_replace(&temporary, &target)?;
            }
        }
        Ok(reference)
    }

    /// Return a complete persistent cache hit without invoking the native icon service.
    pub fn cached(&self, path: &Path) -> std::io::Result<Option<IconReference>> {
        let reference = self.reference(path)?;
        Ok(variants_exist(&self.root, &reference).then_some(reference))
    }

    fn reference(&self, path: &Path) -> std::io::Result<IconReference> {
        let stamp = crate::adapter::file_icon::stamp(&path.metadata()?);
        let mut hash = Sha256::new();
        hash.update(RENDER_VERSION.as_bytes());
        hash.update([0]);
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update([0]);
        hash.update(stamp.as_bytes());
        let key = hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        IconReference::new(key).map_err(std::io::Error::other)
    }
}

fn variants_exist(root: &Path, reference: &IconReference) -> bool {
    [128, 512].iter().all(|size| {
        root.join(reference.key())
            .join(format!("{size}.png"))
            .is_file()
    })
}

fn encode_png(pixels: &[u8], size: u32) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(std::io::Error::other)?;
        writer
            .write_image_data(pixels)
            .map_err(std::io::Error::other)?;
        writer.finish().map_err(std::io::Error::other)?;
    }
    Ok(bytes)
}
