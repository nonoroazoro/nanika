// Script language runtimes are user-installed; the shell interpreter is system-owned.
pub(super) fn interpreter(extension: &str) -> Option<&'static str> {
    match extension {
        "ps1" => Some("pwsh"),
        "py" => Some("python3"),
        "js" => Some("node"),
        "sh" => Some("/bin/bash"),
        _ => None,
    }
}
