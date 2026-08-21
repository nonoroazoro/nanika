//! Minimal child-process fixture for host supervisor tests.

use std::io::{self, stdin, stdout};

use nanika_protocol::{Message, PROTOCOL_NAME, read_frame, write_frame};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let hang_initialize = arguments
        .iter()
        .any(|argument| argument == "--hang-initialize");
    let exit_after_initialize = arguments
        .iter()
        .any(|argument| argument == "--exit-after-initialize");
    if arguments
        .iter()
        .any(|argument| argument == "--write-stderr")
    {
        eprint!("{}", "fixture-stderr-".repeat(1024));
    }
    let mut input = stdin().lock();
    let mut output = stdout().lock();

    while let Some(message) = read_frame(&mut input)? {
        match message {
            Message::Initialize { request_id, .. } => {
                if hang_initialize {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                }
                write_frame(
                    &mut output,
                    &Message::Initialized {
                        request_id,
                        protocol: PROTOCOL_NAME.to_owned(),
                    },
                )?;
                if exit_after_initialize {
                    return Ok(());
                }
            }
            Message::Shutdown { request_id } => {
                write_frame(&mut output, &Message::ShutdownAck { request_id })?;
                return Ok(());
            }
            Message::Query { .. }
            | Message::Snapshot { .. }
            | Message::Invoke { .. }
            | Message::Result { .. }
            | Message::Cancel { .. }
            | Message::Initialized { .. }
            | Message::ShutdownAck { .. }
            | Message::Error { .. } => write_frame(
                &mut output,
                &Message::Error {
                    request_id: None,
                    code: "unsupported_message".to_owned(),
                    message: "fixture accepts only initialize and shutdown".to_owned(),
                },
            )?,
        }
    }

    let _ = io::Write::flush(&mut output);
    Ok(())
}
