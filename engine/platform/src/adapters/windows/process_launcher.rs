use std::sync::mpsc::Receiver;

use nanika_protocol::HostServiceResponse;

use crate::{LauncherCommand, process_launch::process_launch};

pub(crate) fn run(receiver: Receiver<LauncherCommand>, _notifier: ()) {
    while let Ok(command) = receiver.recv() {
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
        }
    }
}
