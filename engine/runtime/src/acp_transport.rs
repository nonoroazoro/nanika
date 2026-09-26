use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

use async_process::{Child, ChildStderr, ChildStdin, ChildStdout};
use futures::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use futures::{Sink, Stream};

use nanika_platform::ExtensionProcessTree;

pub(crate) const ACP_STDERR_LIMIT: usize = 64 * 1024;

pub(crate) fn incoming_lines(
    stdout: ChildStdout,
) -> impl Stream<Item = io::Result<String>> + Send + 'static {
    futures::stream::try_unfold(BufReader::new(stdout), |mut reader| async move {
        read_acp_line(&mut reader)
            .await
            .map(|line| line.map(|line| (line, reader)))
    })
}

pub(crate) fn outgoing_lines(
    stdin: ChildStdin,
) -> impl Sink<String, Error = io::Error> + Send + 'static {
    futures::sink::unfold(stdin, |mut writer, line: String| async move {
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(writer)
    })
}

pub(crate) async fn drain_stderr(
    mut stderr: ChildStderr,
    tail: Arc<Mutex<VecDeque<u8>>>,
    extension_id: String,
) -> io::Result<()> {
    let mut chunk = [0_u8; 4_096];
    loop {
        let read = match stderr.read(&mut chunk).await {
            Ok(0) => return Ok(()),
            Err(error) => {
                return Err(io::Error::other(format!(
                    "could not read ACP extension stderr: {error}"
                )));
            }
            Ok(read) => read,
        };
        tracing::info!(
            %extension_id,
            message = %String::from_utf8_lossy(&chunk[..read]),
            "ACP extension stderr"
        );
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
) -> io::Result<()> {
    let mut first_error = process_tree.terminate(child.id()).err();
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) if first_error.is_none() => first_error = Some(error),
        Err(_) => {}
    }
    child.status().await?;
    first_error.map_or(Ok(()), Err)
}

pub(crate) async fn read_acp_line<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> io::Result<Option<String>> {
    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        return Ok(None);
    }
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Ok(Some(line))
}
