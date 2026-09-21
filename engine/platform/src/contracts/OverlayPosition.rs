/// Absolute physical position consumed by the native viewport adapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayPosition {
    pub x: f32,
    pub y: f32,
}

impl OverlayPosition {
    pub fn scaled(self, scale: f32) -> Self {
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
#[path = "../../tests/contracts/OverlayPosition.rs"]
mod tests;
