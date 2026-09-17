use crate::image_resize::resize_rgba;

pub fn normalize_icon_rgba(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    target_size: u32,
) -> Option<Vec<u8>> {
    if source_width == 0
        || source_height == 0
        || target_size == 0
        || source.len() != rgba_length(source_width, source_height)?
    {
        return None;
    }
    let (left, top, right, bottom) = alpha_bounds(source, source_width, source_height)?;

    let content_width = right - left + 1;
    let content_height = bottom - top + 1;
    // Fill the longest axis. Only aspect-ratio centering may add transparent space.
    let scale = target_size as f32 / content_width.max(content_height) as f32;
    let normalized_width = ((content_width as f32 * scale).round() as u32).clamp(1, target_size);
    let normalized_height = ((content_height as f32 * scale).round() as u32).clamp(1, target_size);
    let cropped = crop_rgba(
        source,
        source_width,
        left,
        top,
        content_width,
        content_height,
    );
    let resized = resize_rgba(
        &cropped,
        content_width,
        content_height,
        normalized_width,
        normalized_height,
    );
    let offset_x = (target_size - normalized_width) / 2;
    let offset_y = (target_size - normalized_height) / 2;
    let mut target = vec![0_u8; rgba_length(target_size, target_size)?];
    for row in 0..normalized_height {
        let source_start = (row * normalized_width * 4) as usize;
        let source_end = source_start + (normalized_width * 4) as usize;
        let target_start = (((offset_y + row) * target_size + offset_x) * 4) as usize;
        let target_end = target_start + (normalized_width * 4) as usize;
        target[target_start..target_end].copy_from_slice(&resized[source_start..source_end]);
    }
    Some(target)
}

fn rgba_length(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)
}

fn alpha_bounds(pixels: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            let alpha = pixels[((y * width + x) * 4 + 3) as usize];
            // Only fully transparent pixels are empty; color is never evidence of background.
            if alpha == 0 {
                continue;
            }
            found = true;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    found.then_some((left, top, right, bottom))
}

fn crop_rgba(
    source: &[u8],
    source_width: u32,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut cropped = vec![0_u8; width.saturating_mul(height).saturating_mul(4) as usize];
    for row in 0..height {
        let source_start = (((top + row) * source_width + left) * 4) as usize;
        let source_end = source_start + (width * 4) as usize;
        let target_start = (row * width * 4) as usize;
        let target_end = target_start + (width * 4) as usize;
        cropped[target_start..target_end].copy_from_slice(&source[source_start..source_end]);
    }
    cropped
}
