use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSScreen};

use crate::{OverlayPosition, PlatformError, centered_position};

pub(crate) fn active_overlay_position(
    width_points: f32,
    height_points: f32,
    current_scale_factor: f32,
) -> Result<OverlayPosition, PlatformError> {
    let marker = MainThreadMarker::new().ok_or_else(|| {
        PlatformError::Message("active monitor placement requires the main thread".to_owned())
    })?;
    let screens = NSScreen::screens(marker);
    let primary_height = screens
        .firstObject()
        .map(|screen| screen.frame().size.height)
        .ok_or_else(|| PlatformError::Message("no macOS screens are available".to_owned()))?;
    let mouse = NSEvent::mouseLocation();
    let screen = screens
        .into_iter()
        .find(|screen| {
            let frame = screen.frame();
            mouse.x >= frame.origin.x
                && mouse.x < frame.origin.x + frame.size.width
                && mouse.y >= frame.origin.y
                && mouse.y < frame.origin.y + frame.size.height
        })
        .ok_or_else(|| PlatformError::Message("pointer monitor was not found".to_owned()))?;
    let visible = screen.visibleFrame();
    let position = centered_position(
        visible.origin.x,
        primary_height - visible.origin.y - visible.size.height,
        visible.origin.x + visible.size.width,
        primary_height - visible.origin.y,
        f64::from(width_points),
        f64::from(height_points),
    );
    Ok(position.scaled(current_scale_factor))
}
