use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_process::{Child, ChildStderr, ChildStdin, ChildStdout};
use futures::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use futures::{Sink, Stream};
use futures_lite::future;

use crate::ExtensionProcessTree;

pub(crate) const ACP_FRAME_LIMIT: usize = 8 * 1024 * 1024;
pub(crate) const ACP_STDERR_LIMIT: usize = 64 * 1024;

pub(crate) fn incoming_lines(
    stdout: ChildStdout,
) -> impl Stream<Item = io::Result<String>> + Send + 'static {
    futures::stream::try_unfold(BufReader::new(stdout), |mut reader| async move {
        read_bounded_line(&mut reader, ACP_FRAME_LIMIT)
            .await
            .map(|line| line.map(|line| (line, reader)))
    })
}

pub(crate) fn outgoing_lines(
    stdin: ChildStdin,
) -> impl Sink<String, Error = io::Error> + Send + 'static {
    futures::sink::unfold(stdin, |mut writer, line: String| async move {
        if line.len() > ACP_FRAME_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ACP outgoing frame exceeds the byte limit",
            ));
        }
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(writer)
    })
}

pub(crate) async fn drain_stderr(mut stderr: ChildStderr, tail: Arc<Mutex<VecDeque<u8>>>) {
    let mut chunk = [0_u8; 4_096];
    loop {
        let read = match stderr.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        let mut tail = tail.lock().unwrap_or_else(|error| error.into_inner());
        tail.extend(&chunk[..read]);
        while tail.len() > ACP_STDERR_LIMIT {
            tail.pop_front();
        }
    }
}

pub(crate) async fn terminate_child(
    child: &mut Child,
    process_tree: &ExtensionProcessTree,
    timeout: Duration,
) -> io::Result<()> {
    let mut first_error = process_tree.terminate(child.id()).err();
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) if first_error.is_none() => first_error = Some(error),
        Err(_) => {}
    }
    let wait = child.status();
    future::race(wait, async move {
        async_io::Timer::after(timeout).await;
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "ACP child did not exit after termination",
        ))
    })
    .await?;
    first_error.map_or(Ok(()), Err)
}

pub(crate) async fn read_bounded_line<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    byte_limit: usize,
) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    loop {
        let (consumed, complete) = {
            let available = reader.fill_buf().await?;
            if available.is_empty() {
                if bytes.is_empty() {
                    return Ok(None);
                }
                return decode_line(bytes).map(Some);
            }
            let newline = available.iter().position(|byte| *byte == b'\n');
            let consumed = newline.map_or(available.len(), |index| index + 1);
            let content = if newline.is_some() {
                &available[..consumed - 1]
            } else {
                &available[..consumed]
            };
            if bytes.len().saturating_add(content.len()) > byte_limit {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ACP incoming frame exceeds the byte limit",
                ));
            }
            bytes.extend_from_slice(content);
            (consumed, newline.is_some())
        };
        reader.consume_unpin(consumed);
        if complete {
            return decode_line(bytes).map(Some);
        }
    }
}

fn decode_line(mut bytes: Vec<u8>) -> io::Result<String> {
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
