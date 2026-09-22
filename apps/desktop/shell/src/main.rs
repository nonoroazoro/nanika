fn main() -> std::process::ExitCode {
    match nanika_desktop::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            nanika_platform::report_fatal_error(&error);
            std::process::ExitCode::FAILURE
        }
    }
}
