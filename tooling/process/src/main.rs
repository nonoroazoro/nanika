use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[path = "OwnedCommand.rs"]
mod owned_command;
use owned_command::OwnedCommand;

fn main() -> std::process::ExitCode {
    match _run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Process supervisor: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn _run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next().ok_or("Expected stdin mode and command")?;
    let managed = match mode.to_str() {
        Some("managed") => true,
        Some("closed") => false,
        _ => return Err("Expected managed or closed stdin".into())
    };
    let program = args.next().ok_or("Expected command")?;
    let mut command = Command::new(program);
    command.args(args).stdin(if managed {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    // stdout is the ownership protocol. Both child streams remain visible in the runner log.
    command.stdout(std::io::stderr()).stderr(Stdio::inherit());
    let mut owned = OwnedCommand::spawn(command)?;
    _report(serde_json::json!({"pid": owned.child.id()}))?;
    let (sender, commands) = mpsc::sync_channel(8);
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    let mut status = None;
    let mut deadline = None;
    let mut forced = false;
    let mut owner_closed = false;
    loop {
        if status.is_none() {
            status = owned.child.try_wait()?;
            if status.is_some() {
                owned.child.stdin.take();
                if deadline.is_none() && !owned.tree.is_empty(owned.child.id())? {
                    // Managed descendants restore borrowed OS state through pipe EOF,
                    // including when their command leader crashes before cleanup.
                    if !managed {
                        owned.tree.request_stop(owned.child.id(), false)?;
                    }
                    deadline = Some(Instant::now() + Duration::from_secs(10));
                }
            }
        }
        if status.is_some() && owned.tree.is_empty(owned.child.id())? {
            break;
        }
        let control = match commands.recv_timeout(Duration::from_millis(25)) {
            Ok(line) => Some(line?),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) if !owner_closed => {
                owner_closed = true;
                Some(if managed { "close" } else { "stop" }.to_owned())
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                std::thread::sleep(Duration::from_millis(25));
                None
            }
        };
        if let Some(control) = control {
            match control.as_str() {
                "shutdown" if managed => {
                    if let Some(input) = &mut owned.child.stdin {
                        input.write_all(b"shutdown\n")?;
                        input.flush()?;
                    }
                    deadline.get_or_insert(Instant::now() + Duration::from_secs(45));
                }
                "close" if managed => {
                    owned.child.stdin.take();
                    deadline.get_or_insert(Instant::now() + Duration::from_secs(10));
                }
                "stop" | "interrupt" => {
                    if deadline.is_none() {
                        owned
                            .tree
                            .request_stop(owned.child.id(), control == "interrupt")?;
                        deadline = Some(Instant::now() + Duration::from_secs(10));
                    }
                }
                _ => return Err(format!("Invalid process control: {control}").into())
            }
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            if forced {
                return Err("Owned process tree did not exit after termination".into());
            }
            owned.tree.terminate(owned.child.id())?;
            forced = true;
            deadline = Some(Instant::now() + Duration::from_secs(5));
        }
    }
    if managed && forced {
        return Err("Cooperative process tree did not finish cleanup before its deadline".into());
    }
    let status = status.ok_or("Missing command exit status")?;
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(windows)]
    let signal: Option<i32> = None;
    _report(serde_json::json!({"exitCode": status.code(), "signalCode": signal}))?;
    Ok(())
}

fn _report(value: serde_json::Value) -> std::io::Result<()> {
    let mut output = std::io::stdout().lock();
    writeln!(output, "{value}")?;
    output.flush()
}
