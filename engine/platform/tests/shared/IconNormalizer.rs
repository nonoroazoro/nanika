use crate::normalize_icon_rgba;

#[test]
fn normalization_fills_the_longest_edge_and_centers_non_square_content() {
    let mut source = vec![0_u8; 8 * 8 * 4];
    for y in 2..6 {
        for x in 3..5 {
            let offset = (y * 8 + x) * 4;
            source[offset..offset + 4].copy_from_slice(&[40, 80, 120, 255]);
        }
    }

    let normalized = normalize_icon_rgba(&source, 8, 8, 32).expect("source icon should normalize");
    let bounds = visible_bounds(&normalized, 32).expect("normalized icon should be visible");

    assert_eq!(bounds, (8, 0, 23, 31));
}

#[test]
fn normalization_preserves_an_empty_transparent_icon() {
    assert!(normalize_icon_rgba(&[0_u8; 4 * 4 * 4], 4, 4, 32).is_none());
}

#[test]
fn square_artwork_has_no_added_margin() {
    let mut source = vec![0; 6 * 6 * 4];
    for y in 1..5 {
        for x in 1..5 {
            source[(y * 6 + x) * 4..(y * 6 + x) * 4 + 4].copy_from_slice(&[40, 80, 120, 255]);
        }
    }
    let normalized = normalize_icon_rgba(&source, 6, 6, 128).unwrap();
    assert_eq!(visible_bounds(&normalized, 128), Some((0, 0, 127, 127)));
    assert!(
        normalized
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [40, 80, 120, 255])
    );
}

#[test]
fn opaque_black_and_white_perimeters_are_preserved_as_artwork() {
    for background in [[0, 0, 0, 255], [255, 255, 255, 255]] {
        let mut source = background.repeat(7 * 7);
        source[(3 * 7 + 3) * 4..(3 * 7 + 3) * 4 + 4].copy_from_slice(&[40, 80, 120, 255]);
        assert_eq!(normalize_icon_rgba(&source, 7, 7, 7).unwrap(), source);
    }
}

#[test]
fn transparent_margin_cropping_ignores_rgb_and_preserves_partial_alpha() {
    let mut source = [255, 255, 255, 0].repeat(6 * 4);
    let artwork = [
        0, 0, 0, 255, 255, 255, 255, 255, 40, 80, 120, 1, 40, 80, 120, 128, 40, 80, 120, 255, 40,
        80, 120, 64, 255, 255, 255, 1, 0, 0, 0, 1,
    ];
    for row in 0..2 {
        let start = ((row + 1) * 6 + 1) * 4;
        source[start..start + 16].copy_from_slice(&artwork[row * 16..row * 16 + 16]);
    }
    let normalized = normalize_icon_rgba(&source, 6, 4, 4).unwrap();
    assert_eq!(&normalized[16..48], &artwork);
    assert!(normalized[..16].iter().all(|byte| *byte == 0));
    assert!(normalized[48..].iter().all(|byte| *byte == 0));
}

#[test]
fn visible_edge_pixels_are_preserved() {
    let source = [
        255, 255, 255, 255, 40, 80, 120, 1, 40, 80, 120, 255, 0, 0, 0, 255,
    ];
    assert_eq!(normalize_icon_rgba(&source, 2, 2, 2).unwrap(), source);
}

#[test]
fn near_white_background_is_not_erased_as_pure_white() {
    let source = [254, 254, 254, 255].repeat(4);
    assert_eq!(normalize_icon_rgba(&source, 2, 2, 2).unwrap(), source);
}

#[test]
fn transparent_images_and_invalid_dimensions_are_rejected() {
    assert!(normalize_icon_rgba(&[0, 0, 0, 0].repeat(4), 2, 2, 32).is_none());
    assert!(normalize_icon_rgba(&[1, 2, 3, 255], 0, 1, 32).is_none());
    assert!(normalize_icon_rgba(&[1, 2, 3, 255], 1, 0, 32).is_none());
    assert!(normalize_icon_rgba(&[1, 2, 3, 255], 1, 1, 0).is_none());
    assert!(normalize_icon_rgba(&[1, 2, 3], 1, 1, 32).is_none());
    assert!(normalize_icon_rgba(&[], u32::MAX, u32::MAX, 32).is_none());
}

fn visible_bounds(pixels: &[u8], size: usize) -> Option<(usize, usize, usize, usize)> {
    let mut left = size;
    let mut top = size;
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for y in 0..size {
        for x in 0..size {
            if pixels[(y * size + x) * 4 + 3] == 0 {
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
