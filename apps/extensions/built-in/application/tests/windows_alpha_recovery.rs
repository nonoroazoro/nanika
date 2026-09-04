use crate::windows_alpha_recovery::recover_rgba;

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
