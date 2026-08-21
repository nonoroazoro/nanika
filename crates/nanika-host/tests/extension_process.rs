use std::path::PathBuf;
use std::time::{Duration, Instant};

use nanika_host::{ExtensionLimits, ExtensionProcess, SupervisorError};

fn fixture_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("test executable path");
    path.pop();
    path.pop();
    path.push(if cfg!(windows) {
        "nanika-extension-fixture.exe"
    } else {
        "nanika-extension-fixture"
    });
    path
}

#[test]
fn fixture_completes_handshake_and_shutdown() {
    let fixture = fixture_path();
    assert!(
        fixture.is_file(),
        "fixture is not built: {}",
        fixture.display()
    );

    let mut extension = ExtensionProcess::spawn(fixture).expect("fixture should spawn");
    extension
        .initialize("initialize-1")
        .expect("fixture should initialize");
    extension
        .shutdown("shutdown-1")
        .expect("fixture should shut down");
}

#[test]
fn initialization_timeout_terminates_the_child() {
    let limits = ExtensionLimits {
        handshake_timeout: Duration::from_millis(50),
        ..ExtensionLimits::default()
    };
    let mut extension =
        ExtensionProcess::spawn_with(fixture_path(), ["--hang-initialize".into()], limits)
            .expect("fixture should spawn");
    assert!(matches!(
        extension.initialize("initialize-timeout"),
        Err(SupervisorError::Timeout("initialization"))
    ));
}

#[test]
fn crashed_extension_restarts_with_a_fixed_budget() {
    let limits = ExtensionLimits {
        max_restarts: 1,
        ..ExtensionLimits::default()
    };
    let mut extension =
        ExtensionProcess::spawn_with(fixture_path(), ["--exit-after-initialize".into()], limits)
            .expect("fixture should spawn");
    extension
        .initialize("initialize-before-crash")
        .expect("fixture should initialize");
    wait_for_exit(&mut extension);
    assert!(
        extension
            .recover_if_exited("initialize-after-restart")
            .expect("first restart should succeed")
    );
    wait_for_exit(&mut extension);
    assert!(matches!(
        extension.recover_if_exited("restart-limit"),
        Err(SupervisorError::RestartLimit)
    ));
}

#[test]
fn stderr_is_drained_into_a_bounded_tail() {
    let limits = ExtensionLimits {
        stderr_tail_bytes: 128,
        ..ExtensionLimits::default()
    };
    let mut extension =
        ExtensionProcess::spawn_with(fixture_path(), ["--write-stderr".into()], limits)
            .expect("fixture should spawn");
    extension
        .initialize("initialize-stderr")
        .expect("fixture should initialize");
    let deadline = Instant::now() + Duration::from_secs(1);
    while extension.stderr_tail().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(extension.stderr_tail().len() <= 128);
    extension
        .shutdown("shutdown-stderr")
        .expect("fixture should shut down");
}

fn wait_for_exit(extension: &mut ExtensionProcess) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while extension.try_wait().expect("process status").is_none() {
        assert!(Instant::now() < deadline, "fixture did not exit");
        std::thread::sleep(Duration::from_millis(1));
    }
}
