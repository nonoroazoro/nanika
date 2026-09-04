use std::path::Path;
use std::process::{Child, Command, Stdio};

use nanika_protocol::{LaunchArguments, LaunchDescriptor};

pub(crate) fn process_launch(descriptor: &LaunchDescriptor) -> std::io::Result<Child> {
    let mut command = match descriptor {
        LaunchDescriptor::Program {
            program,
            arguments,
            working_directory,
        } => {
            if program.trim().is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "launch program is empty",
                ));
            }
            let mut command = Command::new(program);
            match arguments {
                LaunchArguments::Structured { values } => {
                    command.args(values);
                }
                LaunchArguments::WindowsRaw { value } => apply_windows_raw(&mut command, value)?,
            }
            apply_working_directory(&mut command, working_directory.as_deref())?;
            command
        }
        LaunchDescriptor::Shell {
            command,
            working_directory,
        } => {
            if command.trim().is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "shell command is empty",
                ));
            }
            let mut process = shell_command(command);
            apply_working_directory(&mut process, working_directory.as_deref())?;
            process
        }
        LaunchDescriptor::MacApplication { bundle_path } => mac_application(bundle_path)?,
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

fn apply_working_directory(command: &mut Command, directory: Option<&str>) -> std::io::Result<()> {
    let Some(directory) = directory else {
        return Ok(());
    };
    let path = Path::new(directory);
    if !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "launch working directory does not exist: {}",
                path.display()
            ),
        ));
    }
    command.current_dir(path);
    Ok(())
}

#[cfg(windows)]
fn apply_windows_raw(command: &mut Command, value: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    command.raw_arg(value);
    Ok(())
}

#[cfg(not(windows))]
fn apply_windows_raw(_command: &mut Command, _value: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Windows raw arguments are unsupported on this platform",
    ))
}

#[cfg(windows)]
fn shell_command(value: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let interpreter = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(interpreter);
    command.args(["/d", "/s", "/c"]);
    command.raw_arg(format!("\"{value}\""));
    command
}

#[cfg(target_os = "macos")]
fn shell_command(value: &str) -> Command {
    let mut command = Command::new("/bin/zsh");
    command.args(["-lc", value]);
    command
}

#[cfg(not(any(windows, target_os = "macos")))]
fn shell_command(value: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", value]);
    command
}

#[cfg(target_os = "macos")]
fn mac_application(bundle_path: &str) -> std::io::Result<Command> {
    let path = Path::new(bundle_path);
    if !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("application bundle does not exist: {}", path.display()),
        ));
    }
    let mut command = Command::new("/usr/bin/open");
    command.arg(path);
    Ok(command)
}

#[cfg(not(target_os = "macos"))]
fn mac_application(_bundle_path: &str) -> std::io::Result<Command> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "macOS application launch is unsupported on this platform",
    ))
}
