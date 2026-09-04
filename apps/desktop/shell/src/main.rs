fn main() {
    if let Err(error) = nanika_desktop::run() {
        nanika_platform::report_fatal_error(&error);
    }
}
