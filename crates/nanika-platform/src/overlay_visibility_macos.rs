use objc2_app_kit::NSView;
use raw_window_handle::RawWindowHandle;

use crate::PlatformError;

pub(crate) fn apply(window: RawWindowHandle, visible: bool) -> Result<(), PlatformError> {
    let RawWindowHandle::AppKit(handle) = window else {
        return Err(PlatformError::Message(
            "macOS overlay received a non-AppKit window handle".to_owned(),
        ));
    };
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let window = view.window().ok_or_else(|| {
        PlatformError::Message("macOS overlay view is not attached to a window".to_owned())
    })?;

    if visible {
        window.setIgnoresMouseEvents(false);
        window.setAlphaValue(1.0);
    } else {
        window.setAlphaValue(0.0);
        window.setIgnoresMouseEvents(true);
        if window.isKeyWindow() {
            window.orderOut(None);
        }
        window.orderFrontRegardless();
    }
    Ok(())
}
