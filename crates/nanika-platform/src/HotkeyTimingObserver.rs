/// Lifetime guard for native hotkey timing observation.
pub(crate) struct HotkeyTimingObserver {
    handle: *mut std::ffi::c_void,
}

impl HotkeyTimingObserver {
    pub(crate) fn install() -> Option<Self> {
        crate::install_hotkey_timing_observer().map(|handle| Self { handle })
    }
}

impl Drop for HotkeyTimingObserver {
    fn drop(&mut self) {
        crate::uninstall_hotkey_timing_observer(self.handle);
    }
}
