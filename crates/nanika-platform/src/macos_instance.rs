use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::{InstanceRole, PlatformError, PlatformEvent, SingleInstance};

const LOCK_FILE: &str = "nanika.instance.lock";
const SOCKET_FILE: &str = "nanika.instance.sock";
const ACTIVATE_REQUEST: u8 = b'a';
const STOP_REQUEST: u8 = b's';

pub(crate) fn acquire(app_data_root: &Path) -> Result<InstanceRole, PlatformError> {
    std::fs::create_dir_all(app_data_root)?;
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(app_data_root.join(LOCK_FILE))?;
    if unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let error = std::io::Error::last_os_error();
        if matches!(error.kind(), ErrorKind::WouldBlock) {
            return Ok(InstanceRole::Secondary);
        }
        return Err(PlatformError::Io(error));
    }

    let activation_path = app_data_root.join(SOCKET_FILE);
    if activation_path.exists() {
        std::fs::remove_file(&activation_path)?;
    }
    let socket = UnixDatagram::bind(&activation_path)?;
    let (events, event_receiver) = mpsc::sync_channel(8);
    let event_sender = events.clone();
    let event_thread = std::thread::Builder::new()
        .name("nanika-instance-events".to_owned())
        .spawn(move || run_event_loop(socket, events))?;

    Ok(InstanceRole::Primary(SingleInstance {
        events: Some(event_receiver),
        event_sender,
        event_thread: Some(event_thread),
        lock_file,
        activation_path,
    }))
}

pub(crate) fn signal_activate(app_data_root: &Path) -> Result<(), PlatformError> {
    let path = app_data_root.join(SOCKET_FILE);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let socket = UnixDatagram::unbound()?;
        match socket.send_to(&[ACTIVATE_REQUEST], &path) {
            Ok(1) => return Ok(()),
            Ok(_) => return Err(PlatformError::ActivationChannelClosed),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(PlatformError::Io(error)),
        }
    }
}

fn run_event_loop(socket: UnixDatagram, events: mpsc::SyncSender<PlatformEvent>) {
    loop {
        let mut request = [0; 1];
        match socket.recv(&mut request) {
            Ok(1) => {}
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        match request[0] {
            ACTIVATE_REQUEST => {
                let _ = events.try_send(PlatformEvent::Open);
            }
            STOP_REQUEST => break,
            _ => {}
        }
    }
}
