use crate::image_resize::resize_rgba;

#[test]
fn resize_produces_exact_dimensions_and_preserves_opaque_color() {
    let source = [32, 64, 128, 255].repeat(16);
    let resized = resize_rgba(&source, 4, 4, 2, 2);

    assert_eq!(resized.len(), 2 * 2 * 4);
    assert!(
        resized
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [32, 64, 128, 255])
    );
}

#[test]
fn resize_interpolates_transparency_without_dark_fringes() {
    let source = [255, 255, 255, 255, 0, 0, 0, 0];
    let resized = resize_rgba(&source, 2, 1, 1, 1);

    assert_eq!(resized, [255, 255, 255, 128]);
}
