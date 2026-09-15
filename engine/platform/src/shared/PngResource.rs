use std::fs::File;
use std::io::Read;
use std::path::Path;

use nanika_protocol::{MAX_PNG_ENCODED_BYTES, png_dimensions_within_limits};

use crate::PngResourceError;

/// Reads an extension-owned PNG through the shared size, confinement, and image limits.
pub fn read_png_resource(path: &Path, payload_root: &Path) -> Result<Vec<u8>, PngResourceError> {
    let payload_root = payload_root
        .canonicalize()
        .map_err(PngResourceError::from_io)?;
    let path = path.canonicalize().map_err(PngResourceError::from_io)?;
    if !path.starts_with(&payload_root) {
        return Err(PngResourceError::OutsideRoot);
    }
    let mut file = File::open(path).map_err(PngResourceError::from_io)?;
    let metadata = file.metadata().map_err(PngResourceError::from_io)?;
    if !metadata.is_file() {
        return Err(PngResourceError::OutsideRoot);
    }
    if metadata.len() > MAX_PNG_ENCODED_BYTES as u64 {
        return Err(PngResourceError::EncodedSize);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take((MAX_PNG_ENCODED_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(PngResourceError::from_io)?;
    if bytes.len() > MAX_PNG_ENCODED_BYTES {
        return Err(PngResourceError::EncodedSize);
    }
    let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
    let reader = decoder.read_info().map_err(PngResourceError::Decode)?;
    let info = reader.info();
    if !png_dimensions_within_limits(info.width, info.height) {
        return Err(PngResourceError::Dimensions {
            width: info.width,
            height: info.height,
        });
    }
    Ok(bytes)
}
