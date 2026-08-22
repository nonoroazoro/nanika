use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// Platform-owned single-instance guard and activation source.
#[derive(Debug)]
pub struct SingleInstance {
    pub(crate) events: Option<Receiver<crate::PlatformEvent>>,
    pub(crate) event_sender: std::sync::mpsc::SyncSender<crate::PlatformEvent>,
    pub(crate) event_thread: Option<JoinHandle<()>>,
    #[cfg(windows)]
    pub(crate) mutex: isize,
    #[cfg(windows)]
    pub(crate) activation_window: isize,
    #[cfg(target_os = "macos")]
    pub(crate) lock_file: std::fs::File,
    #[cfg(target_os = "macos")]
    pub(crate) activation_path: std::path::PathBuf,
}

impl SingleInstance {
    /// Move the activation stream to the host event bridge.
    pub fn take_events(&mut self) -> Result<Receiver<crate::PlatformEvent>, crate::PlatformError> {
        self.events
            .take()
            .ok_or(crate::PlatformError::ActivationChannelClosed)
    }

    pub fn event_sender(&self) -> std::sync::mpsc::SyncSender<crate::PlatformEvent> {
        self.event_sender.clone()
    }
}

#[cfg(windows)]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        crate::windows_instance::stop(self.activation_window);
        if let Some(thread) = self.event_thread.take() {
            let _ = thread.join();
        }
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(
                self.mutex as windows_sys::Win32::Foundation::HANDLE,
            );
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        use std::io::Write;
        use std::os::fd::AsRawFd;
        use std::os::unix::net::UnixStream;

        if let Ok(mut stream) = UnixStream::connect(&self.activation_path) {
            let _ = stream.write_all(b"s");
        }
        if let Some(thread) = self.event_thread.take() {
            let _ = thread.join();
        }
        unsafe {
            let _ = libc::flock(self.lock_file.as_raw_fd(), libc::LOCK_UN);
        }
        let _ = std::fs::remove_file(&self.activation_path);
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Some(thread) = self.event_thread.take() {
            let _ = thread.join();
        }
    }
}
