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

pub(crate) fn icon_url(extension_id: &str, icon: &nanika_protocol::IconSource) -> String {
    match icon {
        nanika_protocol::IconSource::Package { path } => {
            url(&format!("{extension_id}/package/{path}"))
        }
        nanika_protocol::IconSource::Cache(reference) => {
            url(&format!("{extension_id}/cache/{}/128.png", reference.key()))
        }
    }
}
