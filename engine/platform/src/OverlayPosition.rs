/// Absolute physical position consumed by the native viewport adapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayPosition {
    pub x: f32,
    pub y: f32,
}

impl OverlayPosition {
    #[cfg(any(target_os = "macos", test))]
    pub(crate) fn scaled(self, scale: f32) -> Self {
        Self {
            x: self.x * scale,
            y: self.y * scale,
        }
    }
}

pub(crate) fn centered_position(
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    width: f64,
    height: f64,
) -> OverlayPosition {
    OverlayPosition {
        x: (left + (right - left - width).max(0.0) / 2.0) as f32,
        y: (top + (bottom - top - height).max(0.0) / 2.0) as f32,
    }
}

#[cfg(test)]
mod tests {
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
}
