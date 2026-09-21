use super::centered_position;

#[test]
fn centers_inside_negative_coordinate_work_area() {
    let position = centered_position(-2560.0, 0.0, 0.0, 1400.0, 720.0, 480.0);

    assert_eq!(position.x, -1640.0);
    assert_eq!(position.y, 460.0);
}

#[test]
fn current_window_scale_converts_native_logical_coordinates() {
    let position = centered_position(-1280.0, 0.0, 0.0, 900.0, 720.0, 480.0).scaled(2.0);

    assert_eq!(position.x, -2000.0);
    assert_eq!(position.y, 420.0);
}
