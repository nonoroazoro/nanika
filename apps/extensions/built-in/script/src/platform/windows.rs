// Explicit interpreter names use the host process launcher's normal executable search.
// Missing runtimes are launch errors; no interpreter substitution or policy bypass.
pub(super) fn interpreter(extension: &str) -> Option<&'static str> {
    match extension {
        "ps1" => Some("pwsh.exe"),
        "py" => Some("python.exe"),
        "js" => Some("node.exe"),
        "sh" => Some("bash.exe"),
        _ => None,
    }
}
