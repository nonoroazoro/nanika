use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardLayoutList, HKL
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST
};

pub(crate) fn run(pid: u32) -> Result<(), String> {
    let window = unsafe { GetForegroundWindow() };
    let thread = _owned_thread(window, pid)?;
    let original = unsafe { GetKeyboardLayout(thread) };
    if original.is_null() {
        return Err("Could not capture the current input locale".into());
    }
    let count = unsafe { GetKeyboardLayoutList(0, std::ptr::null_mut()) };
    if count <= 0 {
        return Err("No installed input locales are available".into());
    }
    let mut layouts = vec![std::ptr::null_mut(); count as usize];
    let copied = unsafe { GetKeyboardLayoutList(count, layouts.as_mut_ptr()) };
    if copied <= 0 || copied > count {
        return Err("Could not enumerate installed input locales".into());
    }
    layouts.truncate(copied as usize);
    // HKL's low word is the LANGID. 0x0804 is Chinese (Simplified, China).
    let selected = layouts.into_iter().find(|layout| *layout as usize & 0xffff == 0x0804)
        .ok_or("IME capability unavailable: enable a Simplified Chinese Pinyin input method before running this scenario")?;
    let result = _select(window, pid, selected).and_then(|()| super::_wait_for_owner());
    _select(window, pid, original)
        .map_err(|error| format!("Could not restore the input locale: {error}"))?;
    result
}

fn _owned_thread(window: HWND, pid: u32) -> Result<u32, String> {
    let mut owner = 0;
    let thread = unsafe { GetWindowThreadProcessId(window, &mut owner) };
    if thread == 0 || owner != pid {
        return Err("IME verification window no longer belongs to the owned App".into());
    }
    Ok(thread)
}

fn _select(window: HWND, pid: u32, layout: HKL) -> Result<(), String> {
    let thread = _owned_thread(window, pid)?;
    if unsafe { PostMessageW(window, WM_INPUTLANGCHANGEREQUEST, 0, layout as isize) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while unsafe { GetKeyboardLayout(thread) } != layout {
        _owned_thread(window, pid)?;
        if Instant::now() >= deadline {
            return Err("The App did not accept the requested input locale".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
