pub(crate) fn resize_rgba(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> Vec<u8> {
    if source_width == target_width && source_height == target_height {
        return source.to_vec();
    }
    let mut target = vec![0_u8; (target_width * target_height * 4) as usize];
    for target_y in 0..target_height {
        let source_y = coordinate(target_y, source_height, target_height);
        let top = source_y.floor() as u32;
        let bottom = (top + 1).min(source_height - 1);
        let vertical = source_y - top as f32;
        for target_x in 0..target_width {
            let source_x = coordinate(target_x, source_width, target_width);
            let left = source_x.floor() as u32;
            let right = (left + 1).min(source_width - 1);
            let horizontal = source_x - left as f32;
            let samples = [
                (
                    pixel(source, source_width, left, top),
                    (1.0 - horizontal) * (1.0 - vertical),
                ),
                (
                    pixel(source, source_width, right, top),
                    horizontal * (1.0 - vertical),
                ),
                (
                    pixel(source, source_width, left, bottom),
                    (1.0 - horizontal) * vertical,
                ),
                (
                    pixel(source, source_width, right, bottom),
                    horizontal * vertical,
                ),
            ];
            let mut premultiplied = [0.0_f32; 3];
            let mut alpha = 0.0_f32;
            for (sample, weight) in samples {
                let sample_alpha = f32::from(sample[3]) / 255.0;
                alpha += sample_alpha * weight;
                for channel in 0..3 {
                    premultiplied[channel] += f32::from(sample[channel]) * sample_alpha * weight;
                }
            }
            let offset = ((target_y * target_width + target_x) * 4) as usize;
            if alpha > 0.0 {
                for channel in 0..3 {
                    target[offset + channel] =
                        (premultiplied[channel] / alpha).round().clamp(0.0, 255.0) as u8;
                }
            }
            target[offset + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    target
}

fn coordinate(target: u32, source_size: u32, target_size: u32) -> f32 {
    ((target as f32 + 0.5) * source_size as f32 / target_size as f32 - 0.5)
        .clamp(0.0, source_size.saturating_sub(1) as f32)
}

fn pixel(source: &[u8], width: u32, x: u32, y: u32) -> &[u8] {
    let offset = ((y * width + x) * 4) as usize;
    &source[offset..offset + 4]
}
