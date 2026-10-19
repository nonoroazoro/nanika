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
