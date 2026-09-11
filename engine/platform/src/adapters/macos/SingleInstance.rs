use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// Platform-owned instance guard; dropping it ends the activation source.
#[derive(Debug)]
pub struct SingleInstance {
    pub(super) events: Option<Receiver<crate::PlatformEvent>>,
    pub(super) event_thread: Option<JoinHandle<()>>,
    pub(super) lock_file: std::fs::File,
    pub(super) activation_path: std::path::PathBuf,
}

impl SingleInstance {
    /// Move the activation stream to the host event bridge.
    pub fn take_events(&mut self) -> Result<Receiver<crate::PlatformEvent>, crate::PlatformError> {
        self.events
            .take()
            .ok_or(crate::PlatformError::ActivationChannelClosed)
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        use std::os::unix::net::UnixDatagram;

        let stop_sent = UnixDatagram::unbound()
            .and_then(|socket| socket.send_to(b"s", &self.activation_path))
            .is_ok();
        if stop_sent && let Some(thread) = self.event_thread.take() {
            let _ = thread.join();
        }
        unsafe {
            let _ = libc::flock(self.lock_file.as_raw_fd(), libc::LOCK_UN);
        }
        let _ = std::fs::remove_file(&self.activation_path);
    }
}
