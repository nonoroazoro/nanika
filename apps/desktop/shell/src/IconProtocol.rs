use std::path::PathBuf;

use tauri::http::{Method, Request, Response, StatusCode};

use crate::icon_request::IconRequest;

// Limit queued disk reads independently of catalog size. Senders wait for space;
// the reader processes one cached icon at a time.
const REQUEST_CAPACITY: usize = 400;

pub(crate) struct IconProtocol {
    requests: async_channel::Sender<IconRequest>,
}

impl IconProtocol {
    pub(crate) fn spawn(cache_root: PathBuf, payload_root: PathBuf) -> Result<Self, String> {
        let (requests, receiver) = async_channel::bounded::<IconRequest>(REQUEST_CAPACITY);
        std::thread::Builder::new()
            .name("nanika-icon-protocol".to_owned())
            .spawn(move || {
                while let Ok(request) = receiver.recv_blocking() {
                    request.responder.respond(resolve_request(
                        &cache_root,
                        &payload_root,
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
        // The protocol callback is synchronous. Spawning one async task per
        // request would make the bounded channel ineffective because tasks
        // waiting for capacity could grow without a limit. Refuse overload
        // explicitly so pending requests remain bounded by the queue.
        match self.requests.try_send(request) {
            Ok(()) => {}
            Err(async_channel::TrySendError::Full(request)) => request.responder.respond(response(
                StatusCode::SERVICE_UNAVAILABLE,
                "text/plain",
                Vec::new(),
            )),
            Err(async_channel::TrySendError::Closed(request)) => request.responder.respond(
                response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", Vec::new()),
            ),
        }
    }
}

pub(crate) fn resolve_request(
    cache_root: &std::path::Path,
    payload_root: &std::path::Path,
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
    if segments.len() == 2
        && nanika_foundation::is_valid_extension_id(segments[0])
        && segments[1].ends_with(".png")
        && nanika_protocol::is_valid_resource_path(segments[1])
    {
        let extension_root = payload_root.join(segments[0]);
        let payload = extension_root.join(segments[1]);
        return match nanika_platform::read_png_resource(&payload, &extension_root) {
            Ok(bytes) => response(StatusCode::OK, "image/png", bytes),
            Err(nanika_platform::PngResourceError::NotFound) => {
                response(StatusCode::NOT_FOUND, "text/plain", Vec::new())
            }
            Err(error) => {
                tracing::warn!(
                    extension_id = segments[0],
                    resource = segments[1],
                    %error,
                    "image resource request failed"
                );
                let status = match error {
                    nanika_platform::PngResourceError::OutsideRoot => StatusCode::FORBIDDEN,
                    nanika_platform::PngResourceError::EncodedSize => StatusCode::PAYLOAD_TOO_LARGE,
                    nanika_platform::PngResourceError::Dimensions { .. }
                    | nanika_platform::PngResourceError::Decode(_) => {
                        StatusCode::UNPROCESSABLE_ENTITY
                    }
                    nanika_platform::PngResourceError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    nanika_platform::PngResourceError::NotFound => unreachable!(),
                };
                response(status, "text/plain", Vec::new())
            }
        };
    }
    let [extension_id, icon_key, file_name] = segments.as_slice() else {
        return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new());
    };
    if !nanika_foundation::is_valid_extension_id(extension_id)
        || nanika_protocol::IconReference::new(*icon_key).is_err()
        || !matches!(*file_name, "32.png" | "64.png" | "128.png" | "512.png")
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
