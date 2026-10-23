use super::_handle_process_group_termination;

#[test]
fn process_spawn_excludes_unrelated_pipe_and_preserves_stdio_and_exec_errors() {
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::process::{Command, Stdio};

    // Hold a non-CLOEXEC pipe across spawn to reproduce the pipe/fcntl race
    // deterministically, without relying on concurrent scheduling.
    let mut descriptors = [-1; 2];
    assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
    let reader = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
    let inherited = unsafe { libc::fcntl(descriptors[1], libc::F_DUPFD, 200) };
    assert!(inherited >= 200);
    let inherited = unsafe { OwnedFd::from_raw_fd(inherited) };
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["process_spawn_child_probe", "--nocapture"])
        .env(
            "NANIKA_TEST_INHERITED_PIPE",
            inherited.as_raw_fd().to_string()
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    super::configure_process_command(&mut command);
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"stdio input")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    drop((reader, writer, inherited));
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("stdio input"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("stderr preserved"));

    let mut missing = Command::new("/nanika-missing-executable/child");
    super::configure_process_command(&mut missing);
    assert_eq!(
        missing.spawn().unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}

#[test]
fn process_spawn_child_probe() {
    use std::io::Read;
    let Ok(descriptor) = std::env::var("NANIKA_TEST_INHERITED_PIPE") else {
        return;
    };
    let descriptor: libc::c_int = descriptor.parse().unwrap();
    assert_eq!(unsafe { libc::fcntl(descriptor, libc::F_GETFD) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EBADF)
    );
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    println!("{input}");
    eprintln!("stderr preserved");
}

#[test]
fn process_group_termination_preserves_denial_for_a_live_group() {
    let group = unsafe { libc::getpgrp() };
    assert!(_handle_process_group_termination(group, Ok(())).is_ok());
    assert!(_handle_process_group_termination(group, Err(rustix::io::Errno::SRCH)).is_ok());
    let error = _handle_process_group_termination(group, Err(rustix::io::Errno::PERM))
        .expect_err("permission errors must be propagated");
    assert_eq!(
        error.raw_os_error(),
        Some(rustix::io::Errno::PERM.raw_os_error())
    );
}

#[test]
fn unreaped_exited_group_is_quiescent_without_hiding_live_descendants() {
    use super::{ProcessTree, configure_process_command};
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    for descendant in [false, true] {
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            if descendant {
                "sleep 30 & echo $!; exit 0"
            } else {
                "echo ready; exit 0"
            }
        ]);
        command.stdout(Stdio::piped()).stderr(Stdio::null());
        configure_process_command(&mut command);
        let mut child = command.spawn().unwrap();
        let tree = ProcessTree::attach_std(&child).unwrap();
        let id = child.id();
        let mut ready = String::new();
        std::io::BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let result = std::panic::catch_unwind(|| {
            // WNOWAIT establishes exit without reaping, making Darwin's zombie
            // process-group behavior deterministic instead of relying on timing.
            let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::uninit();
            let waited = unsafe {
                libc::waitid(
                    libc::P_PID,
                    id,
                    info.as_mut_ptr(),
                    libc::WEXITED | libc::WNOWAIT
                )
            };
            assert_eq!(waited, 0);
            assert_eq!(
                super::_group_has_live_members(id.cast_signed()).unwrap(),
                descendant
            );
            assert_eq!(tree.is_empty(id).unwrap(), !descendant);
            let denied =
                _handle_process_group_termination(id.cast_signed(), Err(rustix::io::Errno::PERM));
            assert_eq!(
                denied.is_err(),
                descendant,
                "a live descendant must preserve the original denial"
            );
            if !descendant {
                tree.request_stop(id, false).unwrap();
                tree.terminate(id).unwrap();
            }
        });
        let cleanup = tree.terminate(id);
        child.wait().unwrap();
        cleanup.unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !tree.is_empty(id).unwrap() {
            assert!(
                std::time::Instant::now() < deadline,
                "owned descendants must exit"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        result.unwrap();
    }
}
