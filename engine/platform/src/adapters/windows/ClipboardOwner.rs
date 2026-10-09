use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, HWND_MESSAGE}
};

pub(crate) struct ClipboardOwner {
    _window: HWND
}
impl ClipboardOwner {
    pub(crate) fn new() -> Result<Self, String> {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null()
            )
        };
        if window.is_null() {
            return Err(format!(
                "clipboard owner: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self { _window: window })
    }
    pub(crate) fn handle(&self) -> HWND {
        self._window
    }
}
impl Drop for ClipboardOwner {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self._window);
        }
    }
}
