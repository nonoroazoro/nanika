use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::mpsc::SyncSender;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CWPSTRUCT, CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, WH_CALLWNDPROC,
    WM_SETTINGCHANGE
};

static OBSERVERS: Mutex<BTreeMap<u32, SyncSender<()>>> = Mutex::new(BTreeMap::new());

/// Install on the desktop event thread to observe broadcasts received by its top-level windows.
pub struct LanguageChangeObserver {
    _hook: isize,
    _thread_id: u32
}

impl LanguageChangeObserver {
    pub fn install(wakes: SyncSender<()>) -> Result<Self, String> {
        let thread_id = unsafe { GetCurrentThreadId() };
        {
            let mut observers = OBSERVERS.lock().unwrap_or_else(|error| error.into_inner());
            if observers.contains_key(&thread_id) {
                return Err("A language observer is already installed on this thread.".into());
            }
            observers.insert(thread_id, wakes);
        }
        // WM_SETTINGCHANGE is sent, not necessarily posted. WH_GETMESSAGE and
        // message-only windows do not observe the required top-level broadcasts.
        let hook = unsafe {
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(_observe),
                std::ptr::null_mut(),
                thread_id
            )
        };
        if hook.is_null() {
            let error = std::io::Error::last_os_error();
            OBSERVERS
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&thread_id);
            return Err(format!(
                "Could not observe system language changes: {error}"
            ));
        }
        Ok(Self {
            _hook: hook as isize,
            _thread_id: thread_id
        })
    }
}

impl Drop for LanguageChangeObserver {
    fn drop(&mut self) {
        if unsafe { UnhookWindowsHookEx(self._hook as _) } == 0 {
            tracing::error!(error = %std::io::Error::last_os_error(), "language observer could not detach");
        }
        OBSERVERS
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&self._thread_id);
    }
}

unsafe extern "system" fn _observe(code: i32, word: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && data != 0 {
        let message = unsafe { &*(data as *const CWPSTRUCT) };
        if message.message == WM_SETTINGCHANGE {
            let thread_id = unsafe { GetCurrentThreadId() };
            if let Some(wakes) = OBSERVERS
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .get(&thread_id)
            {
                // Treat the message as a prompt. Its payload does not supply a preferred-language list.
                let _ = wakes.try_send(());
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, word, data) }
}
