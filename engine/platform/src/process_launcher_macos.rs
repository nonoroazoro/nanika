use std::collections::HashMap;
use std::io;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use nanika_protocol::HostServiceResponse;

use crate::{LauncherCommand, process_launch::process_launch};

const COMMAND_EVENT: usize = 1;

pub(crate) fn create_queue() -> io::Result<i32> {
    let queue = unsafe { libc::kqueue() };
    if queue < 0 {
        return Err(io::Error::last_os_error());
    }
    let event = event(
        COMMAND_EVENT,
        libc::EVFILT_USER,
        libc::EV_ADD | libc::EV_CLEAR,
        0,
    );
    if unsafe { libc::kevent(queue, &event, 1, ptr::null_mut(), 0, ptr::null()) } < 0 {
        let error = io::Error::last_os_error();
        unsafe { libc::close(queue) };
        return Err(error);
    }
    Ok(queue)
}

pub(crate) fn wake(queue: i32) -> io::Result<()> {
    let event = event(COMMAND_EVENT, libc::EVFILT_USER, 0, libc::NOTE_TRIGGER);
    if unsafe { libc::kevent(queue, &event, 1, ptr::null_mut(), 0, ptr::null()) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(crate) fn close_queue(queue: i32) {
    unsafe { libc::close(queue) };
}

pub(crate) fn run(receiver: Receiver<LauncherCommand>, queue: i32, shutdown: Arc<AtomicBool>) {
    let mut children = HashMap::<usize, std::process::Child>::new();
    let mut received = event(0, 0, 0, 0);
    'owner: loop {
        let count = unsafe { libc::kevent(queue, ptr::null(), 0, &mut received, 1, ptr::null()) };
        if count < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        let filter = received.filter;
        let identifier = received.ident;
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        if filter == libc::EVFILT_USER {
            loop {
                match receiver.try_recv() {
                    Ok(LauncherCommand::Launch {
                        descriptor,
                        deadline,
                        response,
                    }) => {
                        let result = if Instant::now() >= deadline {
                            Err("process launch request expired before execution".to_owned())
                        } else {
                            process_launch(&descriptor)
                                .and_then(|child| register_child(queue, child, &mut children))
                                .map(|()| HostServiceResponse::Launched)
                                .map_err(|error| error.to_string())
                        };
                        let _ = response.send(result);
                    }
                    Ok(LauncherCommand::Shutdown) | Err(TryRecvError::Disconnected) => {
                        break 'owner;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
        } else if filter == libc::EVFILT_PROC
            && let Some(mut child) = children.remove(&identifier)
        {
            let _ = child.wait();
        }
    }
    for child in children.into_values() {
        let _ = spawn_reaper(child);
    }
    close_queue(queue);
}

fn register_child(
    queue: i32,
    mut child: std::process::Child,
    children: &mut HashMap<usize, std::process::Child>,
) -> io::Result<()> {
    let identifier = child.id() as usize;
    let event = event(
        identifier,
        libc::EVFILT_PROC,
        libc::EV_ADD | libc::EV_ONESHOT,
        libc::NOTE_EXIT,
    );
    if unsafe { libc::kevent(queue, &event, 1, ptr::null_mut(), 0, ptr::null()) } < 0 {
        if child.try_wait()?.is_none() {
            spawn_reaper(child)?;
        }
        return Ok(());
    }
    children.insert(identifier, child);
    Ok(())
}

fn spawn_reaper(mut child: std::process::Child) -> io::Result<()> {
    std::thread::Builder::new()
        .name("nanika-process-reaper-fallback".to_owned())
        .spawn(move || {
            let _ = child.wait();
        })?;
    Ok(())
}

const fn event(ident: usize, filter: i16, flags: u16, fflags: u32) -> libc::kevent {
    libc::kevent {
        ident,
        filter,
        flags,
        fflags,
        data: 0,
        udata: ptr::null_mut(),
    }
}
