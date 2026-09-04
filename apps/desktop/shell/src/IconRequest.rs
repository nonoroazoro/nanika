use tauri::http::Request;

pub(crate) struct IconRequest {
    pub(crate) webview_label: String,
    pub(crate) request: Request<Vec<u8>>,
    pub(crate) responder: tauri::UriSchemeResponder,
}
