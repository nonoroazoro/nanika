use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// Platform-owned instance guard; dropping it ends the activation source.
#[derive(Debug)]
pub struct SingleInstance {
    pub(super) events: Option<Receiver<crate::PlatformEvent>>,
    pub(super) event_thread: Option<JoinHandle<()>>,
    pub(super) mutex: isize,
    pub(super) activation_window: isize,
}

impl SingleInstance {
    pub fn take_events(&mut self) -> Result<Receiver<crate::PlatformEvent>, crate::PlatformError> {
        self.events
            .take()
            .ok_or(crate::PlatformError::ActivationChannelClosed)
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        super::instance::stop(self.activation_window);
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
