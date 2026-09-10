use crate::protocol_input::ProtocolInput;
use nanika_protocol::{FrameError, Message, read_frame};
use std::io::{BufReader, stdin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

// The reader observes explicit cancellation while evaluation owns the main thread.
// Other messages remain ordered and apply backpressure instead of being discarded.
pub(crate) fn spawn() -> std::io::Result<mpsc::Receiver<Result<ProtocolInput, FrameError>>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("nanika-calculator-input".to_owned())
        .spawn(move || {
            let mut input = BufReader::new(stdin().lock());
            let mut active: Option<(String, u64, Arc<AtomicBool>)> = None;
            loop {
                let message = match read_frame(&mut input) {
                    Ok(Some(message)) => message,
                    result => {
                        if let Some((_, _, cancelled)) = &active {
                            cancelled.store(true, Ordering::Release);
                        }
                        if let Err(error) = result {
                            let _ = sender.send(Err(error));
                        }
                        break;
                    }
                };
                if let Message::Cancel {
                    request_id,
                    generation,
                } = &message
                {
                    if let Some((id, current_generation, cancelled)) = &active
                        && id == request_id
                        && current_generation == generation
                    {
                        cancelled.store(true, Ordering::Release);
                    }
                    continue;
                }
                let cancelled = Arc::new(AtomicBool::new(false));
                if let Message::Query {
                    request_id,
                    generation,
                    ..
                } = &message
                {
                    if let Some((_, _, previous)) = active.take() {
                        previous.store(true, Ordering::Release);
                    }
                    active = Some((request_id.clone(), *generation, Arc::clone(&cancelled)));
                }
                let shutdown = matches!(message, Message::Shutdown { .. });
                if shutdown && let Some((_, _, cancelled)) = &active {
                    cancelled.store(true, Ordering::Release);
                }
                if sender
                    .send(Ok(ProtocolInput { message, cancelled }))
                    .is_err()
                    || shutdown
                {
                    break;
                }
            }
        })?;
    Ok(receiver)
}
