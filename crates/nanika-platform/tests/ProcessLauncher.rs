use std::time::{Duration, Instant};

use nanika_platform::ProcessLauncher;
use nanika_protocol::LaunchDescriptor;

#[test]
fn explicit_shell_launch_uses_the_platform_interpreter() {
    let marker = std::env::temp_dir().join(format!("nanika-launch-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let command = if cfg!(windows) {
        format!("echo nanika>\"{}\"", marker.display())
    } else {
        format!("printf nanika > '{}'", marker.display())
    };
    let launcher = ProcessLauncher::spawn().expect("launcher should start");
    launcher
        .launch(LaunchDescriptor::Shell {
            command,
            working_directory: None,
        })
        .expect("shell command should launch");

    let deadline = Instant::now() + Duration::from_secs(10);
    while marker.metadata().map_or(0, |metadata| metadata.len()) == 0 {
        assert!(Instant::now() < deadline, "shell command should complete");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        std::fs::read_to_string(&marker)
            .expect("marker should read")
            .trim(),
        "nanika"
    );
    drop(launcher);
    let _ = std::fs::remove_file(marker);
}

#[test]
fn expired_launch_requests_do_not_execute() {
    let marker =
        std::env::temp_dir().join(format!("nanika-expired-launch-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let command = if cfg!(windows) {
        format!("echo nanika>\"{}\"", marker.display())
    } else {
        format!("printf nanika > '{}'", marker.display())
    };
    let launcher = ProcessLauncher::spawn().expect("launcher should start");
    let result = launcher
        .submit(
            LaunchDescriptor::Shell {
                command,
                working_directory: None,
            },
            Instant::now(),
        )
        .expect("request should enqueue")
        .recv_timeout(Duration::from_secs(1))
        .expect("request should complete");
    assert!(
        result
            .expect_err("expired request should fail")
            .contains("expired")
    );
    assert!(!marker.exists());
}
