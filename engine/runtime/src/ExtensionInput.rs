use std::io::{self, BufWriter};
use std::process::ChildStdin;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

use nanika_protocol::{FrameError, Message, write_frame};

/// One bounded writer per child. The process owner remains able to terminate a blocked pipe.
pub(crate) struct ExtensionInput {
    pub(crate) requests: SyncSender<Message>,
    pub(crate) completions: Receiver<Result<(), FrameError>>,
    _thread: JoinHandle<()>
}

impl ExtensionInput {
    pub(crate) fn spawn(input: ChildStdin) -> io::Result<Self> {
        let (requests, pending) = mpsc::sync_channel(1);
        let (completed, completions) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("nanika-extension-input".to_owned())
            .spawn(move || {
                let mut input = BufWriter::new(input);
                while let Ok(message) = pending.recv() {
                    let result = write_frame(&mut input, &message);
                    let failed = result.is_err();
                    if completed.send(result).is_err() || failed {
                        break;
                    }
                }
                // Discard any partial frame after a failed write. Drop must never retry a flush.
                let _ = input.into_parts();
            })?;
        Ok(Self {
            requests,
            completions,
            _thread: thread
        })
    }

    /// After the last acknowledged write, closing the queue delivers EOF. On abort the owner
    /// must terminate the process tree first, which releases any blocked write before joining.
    pub(crate) fn close(self) -> io::Result<()> {
        drop(self.requests);
        drop(self.completions);
        self._thread
            .join()
            .map_err(|_| io::Error::other("extension input writer panicked"))
    }
}
