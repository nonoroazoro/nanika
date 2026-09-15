#[cfg(target_os = "macos")]
const ORIGIN: &str = "nanika-icon://localhost";
#[cfg(target_os = "windows")]
const ORIGIN: &str = "http://nanika-icon.localhost";

pub(crate) fn origin() -> &'static str {
    ORIGIN
}

pub(crate) fn url(path: &str) -> String {
    format!("{ORIGIN}/{path}")
}
