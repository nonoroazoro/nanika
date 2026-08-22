/// Native platform event translated without domain behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEvent {
    Open,
    Settings,
    RescanApplications,
    Quit,
}
