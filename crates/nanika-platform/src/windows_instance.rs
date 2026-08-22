use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETFOCUS, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    FindWindowExW, GetMessageW, HMENU, HWND_MESSAGE, IDI_APPLICATION, LoadIconW, MF_SEPARATOR,
    MF_STRING, MSG, PostMessageW, RegisterClassW, SetForegroundWindow, TPM_NONOTIFY, TPM_RETURNCMD,
    TrackPopupMenu, WM_APP, WM_CONTEXTMENU, WM_LBUTTONUP, WNDCLASSW,
};

use crate::{InstanceRole, PlatformError, PlatformEvent, SingleInstance};

const ACTIVATE_MESSAGE: u32 = WM_APP + 1;
const STOP_MESSAGE: u32 = WM_APP + 2;
const TRAY_MESSAGE: u32 = WM_APP + 3;
const TRAY_ID: u32 = 1;
const MENU_OPEN: usize = 1;
const MENU_SETTINGS: usize = 2;
const MENU_RESCAN: usize = 3;
const MENU_QUIT: usize = 4;

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

pub(crate) fn acquire(identity: &str) -> Result<InstanceRole, PlatformError> {
    let mutex_name = to_wide(&format!("Local\\{identity}"));
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr()) };
    if mutex.is_null() {
        return Err(os_error("CreateMutexW"));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
        }
        return Ok(InstanceRole::Secondary);
    }

    let (events, event_receiver) = mpsc::sync_channel(8);
    let event_sender = events.clone();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let thread_identity = identity.to_owned();
    let event_thread = match std::thread::Builder::new()
        .name("nanika-instance-events".to_owned())
        .spawn(move || run_event_loop(&thread_identity, events, ready_sender))
    {
        Ok(thread) => thread,
        Err(error) => {
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
            }
            return Err(PlatformError::Io(error));
        }
    };
    let activation_window = match ready_receiver.recv() {
        Ok(Ok(window)) => window,
        Ok(Err(error)) => {
            let _ = event_thread.join();
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
            }
            return Err(error);
        }
        Err(_) => {
            let _ = event_thread.join();
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
            }
            return Err(PlatformError::ActivationChannelClosed);
        }
    };

    Ok(InstanceRole::Primary(SingleInstance {
        events: Some(event_receiver),
        event_sender,
        event_thread: Some(event_thread),
        mutex: mutex as isize,
        activation_window,
    }))
}

pub(crate) fn signal_activate(identity: &str) -> Result<(), PlatformError> {
    let class_name = to_wide(&format!("{identity}.activation"));
    let window_name = to_wide(identity);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let window = unsafe {
            FindWindowExW(
                HWND_MESSAGE,
                std::ptr::null_mut(),
                class_name.as_ptr(),
                window_name.as_ptr(),
            )
        };
        if !window.is_null() {
            if unsafe { PostMessageW(window, ACTIVATE_MESSAGE, 0, 0) } != 0 {
                return Ok(());
            }
            return Err(os_error("PostMessageW"));
        }
        if Instant::now() >= deadline {
            return Err(PlatformError::Timeout("instance activation"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn stop(window: isize) {
    unsafe {
        let _ = PostMessageW(window as HWND, STOP_MESSAGE, 0, 0);
    }
}

fn run_event_loop(
    identity: &str,
    events: mpsc::SyncSender<PlatformEvent>,
    ready: mpsc::SyncSender<Result<isize, PlatformError>>,
) {
    let window = match create_activation_window(identity) {
        Ok(window) => window,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    let tray_added = match add_tray_icon(window) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("Nanika tray icon unavailable: {error}");
            false
        }
    };
    if ready.send(Ok(window as isize)).is_err() {
        unsafe {
            let _ = DestroyWindow(window);
        }
        return;
    }

    loop {
        let mut message = MSG::default();
        let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
        if result <= 0 || message.message == STOP_MESSAGE {
            break;
        }
        if message.message == ACTIVATE_MESSAGE {
            let _ = events.try_send(PlatformEvent::Open);
        } else if message.message == TRAY_MESSAGE {
            match tray_event(message.lParam) {
                WM_LBUTTONUP => {
                    let _ = events.try_send(PlatformEvent::Open);
                }
                WM_CONTEXTMENU => match show_tray_menu(window, message.wParam) {
                    Ok(Some(event)) => {
                        let _ = events.try_send(event);
                    }
                    Ok(None) => {}
                    Err(error) => eprintln!("Nanika tray menu failed: {error}"),
                },
                _ => {}
            }
        }
    }
    if tray_added {
        remove_tray_icon(window);
    }
    unsafe {
        let _ = DestroyWindow(window);
    }
}

fn add_tray_icon(window: HWND) -> Result<(), PlatformError> {
    let icon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    if icon.is_null() {
        return Err(os_error("LoadIconW"));
    }
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: TRAY_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        hIcon: icon,
        ..NOTIFYICONDATAW::default()
    };
    let tip = to_wide("Nanika");
    let length = tip.len().min(data.szTip.len());
    data.szTip[..length].copy_from_slice(&tip[..length]);
    if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
        return Err(PlatformError::Message(
            "Shell_NotifyIconW(NIM_ADD) failed".to_owned(),
        ));
    }
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    if unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) } == 0 {
        remove_tray_icon(window);
        return Err(PlatformError::Message(
            "Shell_NotifyIconW(NIM_SETVERSION) failed".to_owned(),
        ));
    }
    Ok(())
}

fn remove_tray_icon(window: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: TRAY_ID,
        ..NOTIFYICONDATAW::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn show_tray_menu(window: HWND, anchor: usize) -> Result<Option<PlatformEvent>, PlatformError> {
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return Err(os_error("CreatePopupMenu"));
    }
    let result = (|| {
        append_menu_item(menu, MENU_OPEN, "Open Nanika")?;
        append_menu_item(menu, MENU_SETTINGS, "Settings")?;
        append_menu_item(menu, MENU_RESCAN, "Rescan applications")?;
        if unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null()) } == 0 {
            return Err(os_error("AppendMenuW"));
        }
        append_menu_item(menu, MENU_QUIT, "Quit")?;
        let (x, y) = tray_anchor(anchor);
        unsafe {
            let _ = SetForegroundWindow(window);
        }
        let command = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_NONOTIFY,
                x,
                y,
                0,
                window,
                std::ptr::null(),
            )
        } as usize;
        focus_tray_icon(window);
        Ok(match command {
            MENU_OPEN => Some(PlatformEvent::Open),
            MENU_SETTINGS => Some(PlatformEvent::Settings),
            MENU_RESCAN => Some(PlatformEvent::RescanApplications),
            MENU_QUIT => Some(PlatformEvent::Quit),
            _ => None,
        })
    })();
    unsafe {
        let _ = DestroyMenu(menu);
    }
    result
}

fn focus_tray_icon(window: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: TRAY_ID,
        ..NOTIFYICONDATAW::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_SETFOCUS, &data);
    }
}

fn tray_event(value: isize) -> u32 {
    value as u32 & 0xffff
}

fn tray_anchor(value: usize) -> (i32, i32) {
    let x = (value as u16) as i16 as i32;
    let y = ((value >> 16) as u16) as i16 as i32;
    (x, y)
}

fn append_menu_item(menu: HMENU, id: usize, title: &str) -> Result<(), PlatformError> {
    let title = to_wide(title);
    if unsafe { AppendMenuW(menu, MF_STRING, id, title.as_ptr()) } == 0 {
        Err(os_error("AppendMenuW"))
    } else {
        Ok(())
    }
}

fn create_activation_window(identity: &str) -> Result<HWND, PlatformError> {
    let class_name = to_wide(&format!("{identity}.activation"));
    let window_name = to_wide(identity);
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    if module.is_null() {
        return Err(os_error("GetModuleHandleW"));
    }
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module,
        lpszClassName: class_name.as_ptr(),
        ..WNDCLASSW::default()
    };
    if unsafe { RegisterClassW(&window_class) } == 0 {
        let code = unsafe { GetLastError() };
        if code != ERROR_CLASS_ALREADY_EXISTS {
            return Err(PlatformError::OsCode {
                operation: "RegisterClassW",
                code,
            });
        }
    }
    let window = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            module,
            std::ptr::null(),
        )
    };
    if window.is_null() {
        return Err(os_error("CreateWindowExW"));
    }
    Ok(window)
}

fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

fn os_error(operation: &'static str) -> PlatformError {
    PlatformError::OsCode {
        operation,
        code: unsafe { GetLastError() },
    }
}

#[cfg(test)]
mod tests {
    use super::{tray_anchor, tray_event};
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_CONTEXTMENU;

    #[test]
    fn version_four_callback_ignores_the_packed_icon_id() {
        let packed = ((1_u32 << 16) | WM_CONTEXTMENU) as isize;
        assert_eq!(tray_event(packed), WM_CONTEXTMENU);
    }

    #[test]
    fn version_four_anchor_preserves_signed_screen_coordinates() {
        let packed =
            ((20_u32 << 16) | u32::from(u16::from_ne_bytes((-10_i16).to_ne_bytes()))) as usize;
        assert_eq!(tray_anchor(packed), (-10, 20));
    }
}
