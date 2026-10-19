#[cfg(target_os = "macos")]
pub(crate) fn is_open(window: &tauri::WebviewWindow) -> Result<bool, String> {
    use objc2_app_kit::{NSOpenPanel, NSWindow};

    let pointer = window.ns_window().map_err(|error| error.to_string())?;
    // Called on the event-loop thread while the Tauri window owns this NSWindow.
    let native =
        unsafe { pointer.cast::<NSWindow>().as_ref() }.ok_or("Native window is unavailable")?;
    Ok(native
        .attachedSheet()
        .is_some_and(|sheet| sheet.downcast_ref::<NSOpenPanel>().is_some() && sheet.isVisible()))
}

#[cfg(target_os = "windows")]
pub(crate) fn is_open(window: &tauri::WebviewWindow) -> Result<bool, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetLastActivePopup, IsWindowVisible
    };

    let owner = window.hwnd().map_err(|error| error.to_string())?.0;
    let popup = unsafe { GetLastActivePopup(owner) };
    if popup.is_null() || popup == owner || unsafe { IsWindowVisible(popup) } == 0 {
        return Ok(false);
    }
    let mut class = [0_u16; 64];
    let count = unsafe { GetClassNameW(popup, class.as_mut_ptr(), class.len() as i32) };
    if count == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(String::from_utf16_lossy(&class[..count as usize]) == "#32770")
}

#[cfg(target_os = "windows")]
pub(crate) fn query_process(pid: u32, cancel: bool) -> Result<bool, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, PostMessageW, WM_CLOSE};

    let mut query = (pid, Vec::<windows_sys::Win32::Foundation::HWND>::new());
    // Query from the helper process: a native dialog runs a nested message loop,
    // so Tauri's MainEventsCleared snapshot can remain stale until it closes.
    if unsafe { EnumWindows(Some(_collect_dialog), &mut query as *mut _ as isize) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if cancel {
        for window in &query.1 {
            if unsafe { PostMessageW(*window, WM_CLOSE, 0, 0) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
    }
    Ok(!query.1.is_empty())
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn _collect_dialog(
    window: windows_sys::Win32::Foundation::HWND,
    parameter: isize
) -> windows_sys::core::BOOL {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetWindowThreadProcessId, IsWindowVisible
    };
    let query =
        unsafe { &mut *(parameter as *mut (u32, Vec<windows_sys::Win32::Foundation::HWND>)) };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    if pid == query.0 && unsafe { IsWindowVisible(window) } != 0 {
        let mut class = [0_u16; 64];
        let count = unsafe { GetClassNameW(window, class.as_mut_ptr(), class.len() as i32) };
        if count > 0 && String::from_utf16_lossy(&class[..count as usize]) == "#32770" {
            query.1.push(window);
        }
    }
    1
}
