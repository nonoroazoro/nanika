use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::adapter::process_launch::{apply_windows_raw, mac_application, shell_command};
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
