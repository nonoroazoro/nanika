use std::process::Command;

pub(crate) fn apply_windows_raw(command: &mut Command, value: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    command.raw_arg(value);
    Ok(())
}

pub(crate) fn shell_command(value: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let interpreter = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(interpreter);
    command.args(["/d", "/s", "/c"]);
    command.raw_arg(format!("\"{value}\""));
    command
}

pub(crate) fn mac_application(_bundle_path: &str) -> std::io::Result<Command> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "macOS application launch is unsupported on this platform",
    ))
}
