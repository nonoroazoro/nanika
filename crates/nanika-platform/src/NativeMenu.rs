use std::sync::mpsc::SyncSender;

use crate::{PlatformError, PlatformEvent};

/// Main-thread owner for the native menu bar item where the platform requires one.
pub struct NativeMenu {
    #[cfg(target_os = "macos")]
    _inner: crate::MacNativeMenu,
}

impl NativeMenu {
    pub fn new(events: SyncSender<PlatformEvent>) -> Result<Self, PlatformError> {
        #[cfg(target_os = "macos")]
        {
            crate::MacNativeMenu::new(events).map(|inner| Self { _inner: inner })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = events;
            Ok(Self {})
        }
    }
}
