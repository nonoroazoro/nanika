use crate::image_resize::resize_rgba;

const ALPHA_THRESHOLD: u8 = 16;
const OCCUPANCY_NUMERATOR: u32 = 13;
const OCCUPANCY_DENOMINATOR: u32 = 16;

pub(crate) fn normalize_icon_rgba(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    target_size: u32,
) -> Option<Vec<u8>> {
    let target_length = target_size.saturating_mul(target_size).saturating_mul(4) as usize;
    let mut target = vec![0_u8; target_length];
    let (left, top, right, bottom) = alpha_bounds(source, source_width, source_height)?;

    let content_width = right - left + 1;
    let content_height = bottom - top + 1;
    let maximum_extent = target_size
        .saturating_mul(OCCUPANCY_NUMERATOR)
        .div_ceil(OCCUPANCY_DENOMINATOR)
        .max(1);
    let scale = maximum_extent as f32 / content_width.max(content_height) as f32;
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
    for row in 0..normalized_height {
        let source_start = (row * normalized_width * 4) as usize;
        let source_end = source_start + (normalized_width * 4) as usize;
        let target_start = (((offset_y + row) * target_size + offset_x) * 4) as usize;
        let target_end = target_start + (normalized_width * 4) as usize;
        target[target_start..target_end].copy_from_slice(&resized[source_start..source_end]);
    }
    Some(target)
}

fn alpha_bounds(pixels: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    if width == 0
        || height == 0
        || pixels.len() != width.saturating_mul(height).saturating_mul(4) as usize
    {
        return None;
    }
    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            let alpha = pixels[((y * width + x) * 4 + 3) as usize];
            if alpha < ALPHA_THRESHOLD {
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
