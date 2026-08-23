use std::iter::once;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

pub(crate) fn report(message: &str) {
    let title = std::ffi::OsStr::new("Nanika")
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    let message = std::ffi::OsStr::new(message)
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
