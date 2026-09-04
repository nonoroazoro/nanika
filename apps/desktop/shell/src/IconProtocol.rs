use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender, TrySendError};

use tauri::http::{Method, Request, Response, StatusCode};

use crate::icon_request::IconRequest;

const REQUEST_CAPACITY: usize = 64;

pub(crate) struct IconProtocol {
    requests: SyncSender<IconRequest>,
}

impl IconProtocol {
    pub(crate) fn spawn(cache_root: PathBuf) -> Result<Self, String> {
        let (requests, receiver) = mpsc::sync_channel::<IconRequest>(REQUEST_CAPACITY);
        std::thread::Builder::new()
            .name("nanika-icon-protocol".to_owned())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    request.responder.respond(resolve_request(
                        &cache_root,
                        &request.webview_label,
                        &request.request,
                    ));
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self { requests })
    }

    pub(crate) fn respond(
        &self,
        webview_label: &str,
        request: Request<Vec<u8>>,
        responder: tauri::UriSchemeResponder,
    ) {
        let request = IconRequest {
            webview_label: webview_label.to_owned(),
            request,
            responder,
        };
        match self.requests.try_send(request) {
            Ok(()) => {}
            Err(TrySendError::Full(request)) => request.responder.respond(response(
                StatusCode::SERVICE_UNAVAILABLE,
                "text/plain",
                Vec::new(),
            )),
            Err(TrySendError::Disconnected(request)) => request.responder.respond(response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "text/plain",
                Vec::new(),
            )),
        }
    }
}

fn resolve_request(
    cache_root: &std::path::Path,
    webview_label: &str,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if webview_label != "launcher" || request.method() != Method::GET {
        return response(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    }
    let segments = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    let [extension_id, icon_key, file_name] = segments.as_slice() else {
        return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new());
    };
    if !nanika_core::is_valid_extension_id(extension_id)
        || nanika_protocol::IconReference::new(*icon_key).is_err()
        || !matches!(*file_name, "32.png" | "64.png" | "128.png")
    {
        return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new());
    }
    let path = cache_root
        .join("icons")
        .join(extension_id)
        .join(icon_key)
        .join(file_name);
    match std::fs::read(path) {
        Ok(bytes) => response(StatusCode::OK, "image/png", bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            response(StatusCode::NOT_FOUND, "text/plain", Vec::new())
        }
        Err(_) => response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", Vec::new()),
    }
}

fn response(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    let cache_control = if status == StatusCode::OK {
        "private, max-age=31536000, immutable"
    } else {
        "no-store"
    };
    Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header("Cache-Control", cache_control)
        .header("X-Content-Type-Options", "nosniff")
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}
