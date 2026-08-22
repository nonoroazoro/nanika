use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

use crate::PlatformError;

/// A registered global hotkey with replacement rollback.
pub struct HotkeyRegistration {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}

impl HotkeyRegistration {
    pub fn register(hotkey: HotKey) -> Result<Self, PlatformError> {
        let manager = GlobalHotKeyManager::new().map_err(PlatformError::Hotkey)?;
        manager.register(hotkey).map_err(PlatformError::Hotkey)?;
        Ok(Self { manager, hotkey })
    }

    pub fn id(&self) -> u32 {
        self.hotkey.id()
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
