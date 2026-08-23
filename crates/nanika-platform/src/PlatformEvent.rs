/// Native platform event translated without domain behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    Open,
    Settings,
    RescanApplications,
    Quit,
    Failure {
        operation: &'static str,
        message: String,
    },
}
