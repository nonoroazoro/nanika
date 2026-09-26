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

pub(crate) fn icon_url(extension_id: &str, icon: &nanika_protocol::IconSource) -> Option<String> {
    match icon {
        nanika_protocol::IconSource::Empty => None,
        nanika_protocol::IconSource::Package { path } => {
            Some(url(&format!("{extension_id}/package/{path}")))
        }
        nanika_protocol::IconSource::Cache(reference) => Some(url(&format!(
            "{extension_id}/cache/{}/128.png",
            reference.key()
        ))),
    }
}
