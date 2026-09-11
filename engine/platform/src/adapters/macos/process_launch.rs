use std::path::Path;
use std::process::Command;

pub(crate) fn apply_windows_raw(_command: &mut Command, _value: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Windows raw arguments are unsupported on this platform",
    ))
}

pub(crate) fn shell_command(value: &str) -> Command {
    let mut command = Command::new("/bin/zsh");
    command.args(["-lc", value]);
    command
}

pub(crate) fn mac_application(bundle_path: &str) -> std::io::Result<Command> {
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
