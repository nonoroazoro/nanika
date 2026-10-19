use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
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

pub fn acquire(_identity: &str, app_data_root: &Path) -> Result<InstanceRole, PlatformError> {
    std::fs::create_dir_all(app_data_root)?;
    let mut lock_file = OpenOptions::new()
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

    lock_file.set_len(0)?;
    lock_file.write_all(&std::process::id().to_le_bytes())?;

    let activation_path = app_data_root.join(SOCKET_FILE);
    if activation_path.exists() {
        std::fs::remove_file(&activation_path)?;
    }
    let socket = UnixDatagram::bind(&activation_path)?;
    let (events, event_receiver) = mpsc::channel();
    let event_thread = std::thread::Builder::new()
        .name("nanika-instance-events".to_owned())
        .spawn(move || run_event_loop(socket, events))?;

    Ok(InstanceRole::Primary(SingleInstance {
        events: Some(event_receiver),
        event_thread: Some(event_thread),
        lock_file,
        activation_path
    }))
}

pub fn signal_activate(
    _identity: &str,
    app_data_root: &Path,
    expected_pid: Option<u32>
) -> Result<(), PlatformError> {
    let path = app_data_root.join(SOCKET_FILE);
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut request = [0; 5];
    request[0] = ACTIVATE_REQUEST;
    request[1..].copy_from_slice(&expected_pid.unwrap_or(0).to_le_bytes());
    loop {
        let socket = UnixDatagram::unbound()?;
        match socket.connect(&path) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(error) => return Err(PlatformError::Io(error))
        }
        if let Some(expected) = expected_pid {
            let mut recorded = [0; 4];
            std::fs::File::open(app_data_root.join(LOCK_FILE))?.read_exact(&mut recorded)?;
            if u32::from_le_bytes(recorded) != expected {
                return Err(PlatformError::Message(
                    "Activation target changed".to_owned()
                ));
            }
        }
        return match socket.send(&request) {
            Ok(5) => Ok(()),
            Ok(_) => Err(PlatformError::ActivationChannelClosed),
            Err(error) => Err(PlatformError::Io(error))
        };
    }
}

fn run_event_loop(socket: UnixDatagram, events: mpsc::Sender<PlatformEvent>) {
    loop {
        let mut request = [0; 6];
        match socket.recv(&mut request) {
            Ok(1) if request[0] == STOP_REQUEST => break,
            Ok(5) => {}
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break
        };
        let target = u32::from_le_bytes(request[1..5].try_into().expect("four-byte PID"));
        if request[0] == ACTIVATE_REQUEST
            && (target == 0 || target == std::process::id())
            && events.send(PlatformEvent::Open).is_err()
        {
            eprintln!("single-instance event receiver closed during activation");
            break;
        }
    }
}
