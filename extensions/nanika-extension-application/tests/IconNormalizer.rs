use crate::normalize_icon_rgba;

#[test]
fn normalization_uses_a_consistent_visible_extent_and_centers_content() {
    let mut source = vec![0_u8; 8 * 8 * 4];
    for y in 2..6 {
        for x in 3..5 {
            let offset = (y * 8 + x) * 4;
            source[offset..offset + 4].copy_from_slice(&[40, 80, 120, 255]);
        }
    }

    let normalized = normalize_icon_rgba(&source, 8, 8, 32).expect("source icon should normalize");
    let bounds = visible_bounds(&normalized, 32).expect("normalized icon should be visible");

    assert_eq!(bounds, (9, 3, 21, 28));
}

#[test]
fn normalization_preserves_an_empty_transparent_icon() {
    assert!(normalize_icon_rgba(&[0_u8; 4 * 4 * 4], 4, 4, 32).is_none());
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
