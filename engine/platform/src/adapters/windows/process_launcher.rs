use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

use nanika_protocol::HostServiceResponse;

use crate::{LauncherCommand, process_launch::process_launch};

pub(crate) fn run(receiver: Receiver<LauncherCommand>, _notifier: (), shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Acquire) {
        let Ok(command) = receiver.recv() else {
            break;
        };
        match command {
            LauncherCommand::Launch {
                descriptor,
                response,
            } => {
                let result = process_launch(&descriptor)
                    .map(|child| {
                        drop(child);
                        HostServiceResponse::Launched
                    })
                    .map_err(|error| error.to_string());
                if response.send(result).is_err() {
                    tracing::warn!("process launch requester closed before receiving the result");
                }
            }
            LauncherCommand::Reveal { path, response } => {
                let result = super::reveal::reveal(std::path::Path::new(&path))
                    .map(|()| HostServiceResponse::PathRevealed)
                    .map_err(|error| error.to_string());
                if response.send(result).is_err() {
                    tracing::warn!("reveal requester closed before receiving the result");
                }
            }
            LauncherCommand::Shutdown => break,
        }
    }
}
