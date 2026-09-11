/// Lifetime guard for native hotkey timing observation.
pub struct HotkeyTimingObserver {
    handle: usize,
}

impl HotkeyTimingObserver {
    pub fn install() -> Option<Self> {
        super::hotkey_timing::install().map(|handle| Self {
            handle: handle as usize,
        })
    }
}

impl Drop for HotkeyTimingObserver {
    fn drop(&mut self) {
        super::hotkey_timing::uninstall(self.handle as *mut std::ffi::c_void);
    }
}
