use crate::legacy_icon::recover_legacy_rgba;

#[test]
fn legacy_icon_recovery_keeps_opaque_black_pixels() {
    let recovered = recover_legacy_rgba(vec![0, 0, 0, 0], &[0, 0, 0, 0]);

    assert_eq!(recovered, [0, 0, 0, 255]);
}

#[test]
fn legacy_icon_recovery_keeps_transparent_pixels_transparent() {
    let recovered = recover_legacy_rgba(vec![0, 0, 0, 0], &[255, 255, 255, 0]);

    assert_eq!(recovered, [0, 0, 0, 0]);
}

#[test]
fn legacy_icon_recovery_unpremultiplies_translucent_color() {
    let recovered = recover_legacy_rgba(vec![0, 0, 128, 0], &[127, 127, 255, 0]);

    assert_eq!(recovered, [255, 0, 0, 128]);
}
