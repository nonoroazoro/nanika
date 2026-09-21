#[test]
fn converts_premultiplied_channels_without_darkening_translucent_edges() {
    let mut pixels = [64, 32, 0, 128, 17, 23, 9, 0, 10, 20, 30, 255, 1, 0, 0, 1];
    super::unpremultiply(&mut pixels);
    assert_eq!(
        pixels,
        [128, 64, 0, 128, 0, 0, 0, 0, 10, 20, 30, 255, 255, 0, 0, 1]
    );
}
