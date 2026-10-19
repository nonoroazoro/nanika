fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.is_empty() {
        return match _activate(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    match nanika_desktop::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            nanika_platform::report_fatal_error(&error);
            std::process::ExitCode::FAILURE
        }
    }
}

fn _activate(args: &[std::ffi::OsString]) -> Result<(), String> {
    let pid = match args {
        [command, pid] if command == "--activate" => {
            pid.to_str().and_then(|pid| pid.parse::<u32>().ok())
        }
        _ => None
    }
    .filter(|pid| *pid > 0)
    .ok_or_else(|| "Usage: nanika-desktop --activate <pid>".to_owned())?;
    let paths = nanika_storage::NanikaPaths::discover()
        .ok_or_else(|| "Nanika could not resolve its data directories".to_owned())?;
    nanika_platform::signal_activate(
        nanika_foundation::PROJECT_IDENTITY.bundle_id,
        paths.app_data_root(),
        Some(pid)
    )
    .map_err(|error| error.to_string())
}
