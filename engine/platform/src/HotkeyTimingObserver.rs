/// Lifetime guard for native hotkey timing observation.
pub struct HotkeyTimingObserver {
    handle: usize,
}

impl HotkeyTimingObserver {
    pub fn install() -> Option<Self> {
        crate::install_hotkey_timing_observer().map(|handle| Self {
            handle: handle as usize,
        })
    }
}

impl Drop for HotkeyTimingObserver {
    fn drop(&mut self) {
        crate::uninstall_hotkey_timing_observer(self.handle as *mut std::ffi::c_void);
    }
}
