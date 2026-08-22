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
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW, GetMessageW, HWND_MESSAGE, MSG,
    PostMessageW, RegisterClassW, WM_APP, WNDCLASSW,
};

use crate::{InstanceRole, PlatformError, SingleInstance};

const ACTIVATE_MESSAGE: u32 = WM_APP + 1;
const STOP_MESSAGE: u32 = WM_APP + 2;

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

    let (activations, activation_receiver) = mpsc::sync_channel(1);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let thread_identity = identity.to_owned();
    let event_thread = match std::thread::Builder::new()
        .name("nanika-instance-events".to_owned())
        .spawn(move || run_event_loop(&thread_identity, activations, ready_sender))
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
        activations: Some(activation_receiver),
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
    activations: mpsc::SyncSender<()>,
    ready: mpsc::SyncSender<Result<isize, PlatformError>>,
) {
    let window = match create_activation_window(identity) {
        Ok(window) => window,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
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
            let _ = activations.try_send(());
        }
    }
    unsafe {
        let _ = DestroyWindow(window);
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
