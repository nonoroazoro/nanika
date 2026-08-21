//! Platform adapter boundary.

#![allow(unsafe_code)]

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::iter::once;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

/// Platform-independent adapter error.
#[derive(Debug)]
pub enum PlatformError {
    Unsupported(&'static str),
    OsCode { operation: &'static str, code: u32 },
    Hotkey(global_hotkey::Error),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(operation) => {
                write!(formatter, "unsupported platform operation: {operation}")
            }
            Self::OsCode { operation, code } => {
                write!(formatter, "{operation} failed with OS error {code}")
            }
            Self::Hotkey(error) => write!(formatter, "hotkey error: {error}"),
        }
    }
}

impl std::error::Error for PlatformError {}

/// The target platform selected by the current build.
pub const fn target_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unsupported"
    }
}

/// A registered global hotkey with replacement rollback.
pub struct HotkeyRegistration {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}

impl HotkeyRegistration {
    pub fn register(hotkey: HotKey) -> Result<Self, PlatformError> {
        let manager = GlobalHotKeyManager::new().map_err(PlatformError::Hotkey)?;
        manager.register(hotkey).map_err(PlatformError::Hotkey)?;
        Ok(Self { manager, hotkey })
    }

    pub fn id(&self) -> u32 {
        self.hotkey.id()
    }

    pub fn replace(&mut self, replacement: HotKey) -> Result<(), PlatformError> {
        self.manager
            .register(replacement)
            .map_err(PlatformError::Hotkey)?;
        if let Err(error) = self.manager.unregister(self.hotkey) {
            let _ = self.manager.unregister(replacement);
            return Err(PlatformError::Hotkey(error));
        }
        self.hotkey = replacement;
        Ok(())
    }
}

impl Drop for HotkeyRegistration {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey);
    }
}

/// The result of trying to become the host instance for the current session.
#[derive(Debug)]
pub enum InstanceRole {
    Primary(SingleInstance),
    Secondary,
}

/// Platform-owned single-instance guard.
#[derive(Debug)]
pub struct SingleInstance {
    #[cfg(windows)]
    mutex: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(windows)]
    activation_window: windows_sys::Win32::Foundation::HWND,
    #[cfg(target_os = "macos")]
    lock_file: std::fs::File,
    #[cfg(target_os = "macos")]
    activation_listener: std::os::unix::net::UnixListener,
    #[cfg(target_os = "macos")]
    activation_path: std::path::PathBuf,
}

/// Acquire the current user's Nanika host instance.
pub fn acquire_instance(
    identity: &str,
    app_data_root: &Path,
) -> Result<InstanceRole, PlatformError> {
    #[cfg(windows)]
    {
        let _ = app_data_root;
        windows_instance::acquire(identity)
    }

    #[cfg(target_os = "macos")]
    {
        macos_instance::acquire(identity, app_data_root)
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (identity, app_data_root);
        Err(PlatformError::Unsupported("single instance"))
    }
}

/// Signal an already-running host to activate its overlay.
pub fn signal_activate(identity: &str, app_data_root: &Path) -> Result<(), PlatformError> {
    #[cfg(windows)]
    {
        let _ = app_data_root;
        windows_instance::signal_activate(identity)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = identity;
        macos_instance::signal_activate(app_data_root)
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (identity, app_data_root);
        Err(PlatformError::Unsupported("instance activation"))
    }
}

#[cfg(windows)]
impl SingleInstance {
    /// Wait for the next activation request.
    pub fn wait_for_activation(&self, timeout_ms: u32) -> Result<bool, PlatformError> {
        windows_instance::wait_for_activation(self.activation_window, timeout_ms)
    }
}

#[cfg(windows)]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ =
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(self.activation_window);
            let _ = windows_sys::Win32::Foundation::CloseHandle(self.mutex);
        }
    }
}

#[cfg(target_os = "macos")]
impl SingleInstance {
    /// Wait for the next activation request.
    pub fn wait_for_activation(&self, timeout_ms: u32) -> Result<bool, PlatformError> {
        macos_instance::wait_for_activation(&self.activation_listener, timeout_ms)
    }
}

#[cfg(target_os = "macos")]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        unsafe {
            let _ = libc::flock(self.lock_file.as_raw_fd(), libc::LOCK_UN);
        }
        let _ = std::fs::remove_file(&self.activation_path);
    }
}

#[cfg(windows)]
mod windows_instance {
    use super::{InstanceRole, PlatformError, SingleInstance, to_wide};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, FindWindowExW, HWND_MESSAGE, MSG, PM_REMOVE, PeekMessageW,
        PostMessageW, RegisterClassW, WM_APP, WNDCLASSW,
    };

    const ACTIVATE_MESSAGE: u32 = WM_APP + 1;

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    pub(super) fn acquire(identity: &str) -> Result<InstanceRole, PlatformError> {
        let mutex_name = to_wide(&format!("Local\\{identity}"));
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
        if mutex.is_null() {
            return Err(PlatformError::OsCode {
                operation: "CreateMutexW",
                code: unsafe { GetLastError() },
            });
        }

        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
            }
            return Ok(InstanceRole::Secondary);
        }

        let class_name = to_wide(&format!("{identity}.activation"));
        let window_name = to_wide(identity);
        let module = unsafe { GetModuleHandleW(std::ptr::null()) };
        let window_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: module,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        if unsafe { RegisterClassW(&window_class) } == 0 {
            let error = unsafe { GetLastError() };
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
            }
            return Err(PlatformError::OsCode {
                operation: "RegisterClassW",
                code: error,
            });
        }

        let activation_window = unsafe {
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
        if activation_window.is_null() {
            let error = unsafe { GetLastError() };
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(mutex);
            }
            return Err(PlatformError::OsCode {
                operation: "CreateWindowExW",
                code: error,
            });
        }

        Ok(InstanceRole::Primary(SingleInstance {
            mutex,
            activation_window,
        }))
    }

    pub(super) fn signal_activate(identity: &str) -> Result<(), PlatformError> {
        let class_name = to_wide(&format!("{identity}.activation"));
        let window_name = to_wide(identity);
        let window = unsafe {
            FindWindowExW(
                HWND_MESSAGE,
                std::ptr::null_mut(),
                class_name.as_ptr(),
                window_name.as_ptr(),
            )
        };
        if window.is_null() {
            return Err(PlatformError::OsCode {
                operation: "FindWindowExW",
                code: unsafe { windows_sys::Win32::Foundation::GetLastError() },
            });
        }
        if unsafe { PostMessageW(window, ACTIVATE_MESSAGE, 0, 0) } == 0 {
            return Err(PlatformError::OsCode {
                operation: "PostMessageW",
                code: unsafe { windows_sys::Win32::Foundation::GetLastError() },
            });
        }
        Ok(())
    }

    pub(super) fn wait_for_activation(
        window: HWND,
        timeout_ms: u32,
    ) -> Result<bool, PlatformError> {
        let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        loop {
            let mut message = MSG::default();
            if unsafe {
                PeekMessageW(
                    &mut message,
                    window,
                    ACTIVATE_MESSAGE,
                    ACTIVATE_MESSAGE,
                    PM_REMOVE,
                )
            } != 0
            {
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[allow(dead_code)]
    fn _handle_type_is_send_sync(_: HANDLE) {}
}

#[cfg(windows)]
fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

#[cfg(target_os = "macos")]
mod macos_instance {
    use std::fs::OpenOptions;
    use std::io::{ErrorKind, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{InstanceRole, PlatformError, SingleInstance};

    const LOCK_FILE: &str = "nanika.instance.lock";
    const SOCKET_FILE: &str = "nanika.instance.sock";

    pub(super) fn acquire(
        _identity: &str,
        app_data_root: &Path,
    ) -> Result<InstanceRole, PlatformError> {
        std::fs::create_dir_all(app_data_root)
            .map_err(|error| os_error("create instance directory", error))?;
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(app_data_root.join(LOCK_FILE))
            .map_err(|error| os_error("open instance lock", error))?;
        if unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            if matches!(error.kind(), ErrorKind::WouldBlock) {
                return Ok(InstanceRole::Secondary);
            }
            return Err(os_error("flock", error));
        }

        let activation_path = app_data_root.join(SOCKET_FILE);
        if activation_path.exists() {
            std::fs::remove_file(&activation_path)
                .map_err(|error| os_error("remove stale activation socket", error))?;
        }
        let activation_listener = UnixListener::bind(&activation_path)
            .map_err(|error| os_error("bind activation socket", error))?;
        activation_listener
            .set_nonblocking(true)
            .map_err(|error| os_error("configure activation socket", error))?;

        Ok(InstanceRole::Primary(SingleInstance {
            lock_file,
            activation_listener,
            activation_path,
        }))
    }

    pub(super) fn signal_activate(app_data_root: &Path) -> Result<(), PlatformError> {
        let mut stream = UnixStream::connect(app_data_root.join(SOCKET_FILE))
            .map_err(|error| os_error("connect activation socket", error))?;
        stream
            .write_all(b"activate")
            .map_err(|error| os_error("write activation request", error))
    }

    pub(super) fn wait_for_activation(
        listener: &UnixListener,
        timeout_ms: u32,
    ) -> Result<bool, PlatformError> {
        let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0; 8];
                    stream
                        .read_exact(&mut request)
                        .map_err(|error| os_error("read activation request", error))?;
                    return Ok(request == *b"activate");
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Ok(false);
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(os_error("accept activation request", error)),
            }
        }
    }

    fn os_error(operation: &'static str, error: std::io::Error) -> PlatformError {
        PlatformError::OsCode {
            operation,
            code: error.raw_os_error().unwrap_or_default() as u32,
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{InstanceRole, acquire_instance, signal_activate};

    #[test]
    fn second_launch_signals_the_primary() {
        let identity = format!("com.nanika.test.{}", std::process::id());
        let root = std::env::temp_dir();
        let primary = acquire_instance(&identity, &root).expect("primary should acquire");
        let instance = match primary {
            InstanceRole::Primary(instance) => instance,
            InstanceRole::Secondary => panic!("first launch became secondary"),
        };

        assert!(matches!(
            acquire_instance(&identity, &root).expect("second launch should acquire role"),
            InstanceRole::Secondary
        ));
        signal_activate(&identity, &root).expect("second launch should signal activation");
        assert!(
            instance
                .wait_for_activation(1000)
                .expect("wait should succeed")
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::{InstanceRole, acquire_instance, signal_activate};

    #[test]
    fn second_launch_signals_the_primary() {
        let identity = format!("com.nanika.test.{}", std::process::id());
        let root = std::env::temp_dir().join(&identity);
        let primary = acquire_instance(&identity, &root).expect("primary should acquire");
        let instance = match primary {
            InstanceRole::Primary(instance) => instance,
            InstanceRole::Secondary => panic!("first launch became secondary"),
        };

        assert!(matches!(
            acquire_instance(&identity, &root).expect("second launch should acquire role"),
            InstanceRole::Secondary
        ));
        signal_activate(&identity, &root).expect("second launch should signal activation");
        assert!(
            instance
                .wait_for_activation(1000)
                .expect("wait should succeed")
        );
        drop(instance);
        let _ = std::fs::remove_file(root.join("nanika.instance.lock"));
        let _ = std::fs::remove_dir(root);
    }
}
