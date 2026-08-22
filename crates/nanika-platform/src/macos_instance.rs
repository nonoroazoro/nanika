use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
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
    let listener = UnixListener::bind(&activation_path)?;
    let (events, event_receiver) = mpsc::sync_channel(8);
    let event_sender = events.clone();
    let event_thread = std::thread::Builder::new()
        .name("nanika-instance-events".to_owned())
        .spawn(move || run_event_loop(listener, events))?;

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
        match UnixStream::connect(&path) {
            Ok(mut stream) => {
                stream.write_all(&[ACTIVATE_REQUEST])?;
                return Ok(());
            }
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

fn run_event_loop(listener: UnixListener, events: mpsc::SyncSender<PlatformEvent>) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            break;
        };
        if stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .is_err()
        {
            continue;
        }
        let mut request = [0; 1];
        let Ok(()) = stream.read_exact(&mut request) else {
            continue;
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
