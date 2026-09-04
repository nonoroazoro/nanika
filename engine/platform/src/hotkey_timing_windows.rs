use std::ffi::c_void;
use std::time::Duration;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, MSG, SetWindowsHookExW, UnhookWindowsHookEx, WH_GETMESSAGE, WM_HOTKEY,
};

const MAXIMUM_DELIVERY_DELAY_MS: u32 = 60_000;

pub(crate) fn install() -> Option<*mut c_void> {
    let hook = unsafe {
        SetWindowsHookExW(
            WH_GETMESSAGE,
            Some(observe_message),
            std::ptr::null_mut(),
            GetCurrentThreadId(),
        )
    };
    (!hook.is_null()).then_some(hook.cast())
}

pub(crate) fn uninstall(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            UnhookWindowsHookEx(handle.cast::<c_void>() as HHOOK);
        }
    }
}

unsafe extern "system" fn observe_message(code: i32, word: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && data != 0 {
        let message = unsafe { &*(data as *const MSG) };
        if message.message == WM_HOTKEY {
            let current = unsafe { GetTickCount64() } as u32;
            let delay_ms = current.wrapping_sub(message.time);
            if delay_ms <= MAXIMUM_DELIVERY_DELAY_MS {
                crate::record_hotkey_delivery(
                    message.wParam as u32,
                    Duration::from_millis(delay_ms.into()),
                );
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, word, data) }
}
