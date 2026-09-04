pub(crate) fn recover_rgba(mut black: Vec<u8>, white: &[u8]) -> Vec<u8> {
    for (pixel, white_pixel) in black
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(white.as_chunks::<4>().0)
    {
        pixel.swap(0, 2);
        let difference = (0..3)
            .map(|channel| white_pixel[2 - channel].saturating_sub(pixel[channel]))
            .map(u16::from)
            .sum::<u16>()
            / 3;
        let alpha = u8::MAX.saturating_sub(difference as u8);
        pixel[3] = alpha;
        if alpha > 0 && alpha < u8::MAX {
            let alpha = u16::from(alpha);
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * 255) / alpha).min(255) as u8;
            }
        }
    }
    black
}
