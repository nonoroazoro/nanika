use global_hotkey::GlobalHotKeyEvent;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::HotKeyState;
use global_hotkey::hotkey::HotKey;

use std::time::Duration;

use crate::{HotkeyTimingObserver, PlatformError};

/// A registered global hotkey with replacement rollback.
pub struct HotkeyRegistration {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    _timing_observer: Option<HotkeyTimingObserver>,
}

impl HotkeyRegistration {
    pub fn set_event_handler<F>(handler: F)
    where
        F: Fn(GlobalHotKeyEvent, Option<Duration>) + Send + Sync + 'static,
    {
        crate::set_hotkey_event_handler(handler);
    }

    pub fn register(hotkey: HotKey) -> Result<Self, PlatformError> {
        let manager = GlobalHotKeyManager::new().map_err(PlatformError::Hotkey)?;
        manager.register(hotkey).map_err(PlatformError::Hotkey)?;
        let timing_observer = HotkeyTimingObserver::install();
        Ok(Self {
            manager,
            hotkey,
            _timing_observer: timing_observer,
        })
    }

    pub fn id(&self) -> u32 {
        self.hotkey.id()
    }

    pub fn is_activation(&self, event: &GlobalHotKeyEvent) -> bool {
        self.hotkey.id() == event.id && event.state == HotKeyState::Pressed
    }

    pub fn replace(&mut self, replacement: HotKey) -> Result<(), PlatformError> {
        self.manager
            .register(replacement)
            .map_err(PlatformError::Hotkey)?;
        if let Err(error) = self.manager.unregister(self.hotkey) {
            let _ = self.manager.unregister(replacement);
            return Err(PlatformError::Hotkey(error));
        }
        self.hotkey = replacement;
        Ok(())
    }
}

impl Drop for HotkeyRegistration {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey);
    }
}
