use std::time::{Duration, Instant};

use nanika_platform::ProcessLauncher;
use nanika_protocol::LaunchDescriptor;

#[cfg(windows)]
#[test]
fn shortcut_launch_preserves_arguments_and_working_directory() {
    let root = std::env::temp_dir().join(format!("nanika shortcut 中文 {}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let marker = root.join("marker.txt");
    let _ = std::fs::remove_file(&marker);
    std::fs::write(
        root.join("record.cmd"),
        "@echo off\r\n>marker.txt echo %CD%\r\n",
    )
    .unwrap();
    let shortcut = root.join("Record.lnk");
    let script = r#"
$ErrorActionPreference = 'Stop'
$link = (New-Object -ComObject WScript.Shell).CreateShortcut($env:NANIKA_SHORTCUT_TEST_PATH)
$link.TargetPath = $env:ComSpec
$link.Arguments = '/d /c record.cmd'
$link.WorkingDirectory = $env:NANIKA_SHORTCUT_TEST_ROOT
$link.WindowStyle = 7
$link.Save()
"#;
    let setup = std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("NANIKA_SHORTCUT_TEST_PATH", &shortcut)
        .env("NANIKA_SHORTCUT_TEST_ROOT", &root)
        .output()
        .unwrap();
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let launcher = ProcessLauncher::spawn().unwrap();
    launcher
        .launch(LaunchDescriptor::WindowsApplication {
            path: shortcut.to_string_lossy().into_owned(),
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while marker.metadata().map_or(0, |metadata| metadata.len()) == 0 {
        assert!(
            Instant::now() < deadline,
            "shortcut command should complete"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    // The command interpreter writes the active OEM encoding; an ASCII suffix verifies the chosen
    // working directory while the shortcut path itself exercises Unicode.
    let contents = std::fs::read(&marker).unwrap();
    assert!(
        String::from_utf8_lossy(&contents)
            .trim()
            .ends_with(&std::process::id().to_string())
    );
    drop(launcher);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn invalid_or_deleted_shortcuts_return_errors() {
    let launcher = ProcessLauncher::spawn().unwrap();
    for path in [
        "relative.lnk",
        "C:\\Windows\\not-an-application.txt",
        "C:\\bad\0.lnk",
    ] {
        assert!(
            launcher
                .launch(LaunchDescriptor::WindowsApplication {
                    path: path.to_owned()
                })
                .is_err()
        );
    }
    let missing = std::env::temp_dir().join(format!("nanika-missing-{}.lnk", std::process::id()));
    assert!(
        launcher
            .launch(LaunchDescriptor::WindowsApplication {
                path: missing.to_string_lossy().into_owned()
            })
            .is_err()
    );
}

#[cfg(windows)]
#[test]
fn native_application_accepts_canonical_executable_paths() {
    // No DLL or entry point is supplied, so the library launcher exits without an action.
    let program = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap())
        .join("System32/rundll32.exe")
        .canonicalize()
        .unwrap();
    ProcessLauncher::spawn()
        .unwrap()
        .launch(LaunchDescriptor::WindowsApplication {
            path: program.to_string_lossy().into_owned(),
        })
        .unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn windows_application_is_explicitly_unsupported() {
    let launcher = ProcessLauncher::spawn().unwrap();
    let error = launcher
        .launch(LaunchDescriptor::WindowsApplication {
            path: "C:\\Example.lnk".to_owned(),
        })
        .unwrap_err();
    assert!(error.contains("unsupported"));
}

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
