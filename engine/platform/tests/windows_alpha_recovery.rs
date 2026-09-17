use crate::windows_alpha_recovery::{recover_rgba, unpremultiply_bgra_to_rgba};

#[test]
fn shell_bitmap_conversion_restores_straight_rgba() {
    let mut pixels = vec![32, 64, 128, 128, 9, 8, 7, 0, 3, 2, 1, 255];

    unpremultiply_bgra_to_rgba(&mut pixels);

    assert_eq!(pixels, [255, 127, 63, 128, 0, 0, 0, 0, 1, 2, 3, 255]);
}

#[test]
fn recovery_keeps_opaque_black_pixels() {
    let recovered = recover_rgba(vec![0, 0, 0, 0], &[0, 0, 0, 0]);

    assert_eq!(recovered, [0, 0, 0, 255]);
}

#[test]
fn recovery_keeps_transparent_pixels_transparent() {
    let recovered = recover_rgba(vec![0, 0, 0, 0], &[255, 255, 255, 0]);

    assert_eq!(recovered, [0, 0, 0, 0]);
}

#[test]
fn recovery_unpremultiplies_translucent_color() {
    let recovered = recover_rgba(vec![0, 0, 128, 0], &[127, 127, 255, 0]);

    assert_eq!(recovered, [255, 0, 0, 128]);
}
