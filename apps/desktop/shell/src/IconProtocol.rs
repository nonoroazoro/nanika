use std::path::PathBuf;

use tauri::http::{Method, Request, Response, StatusCode};

use crate::icon_request::IconRequest;

// Bound queued disk reads independently of catalog size; reject overload without
// spawning unbounded tasks. All package, cache and payload reads share this worker.
const REQUEST_CAPACITY: usize = 400;

pub(crate) struct IconProtocol {
    requests: async_channel::Sender<IconRequest>,
    packages: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, PathBuf>>>,
    thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl IconProtocol {
    pub(crate) fn spawn(cache_root: PathBuf, payload_root: PathBuf) -> Result<Self, String> {
        let (requests, receiver) = async_channel::bounded::<IconRequest>(REQUEST_CAPACITY);
        let packages =
            std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        let worker_packages = std::sync::Arc::clone(&packages);
        let thread = std::thread::Builder::new()
            .name("nanika-icon-protocol".to_owned())
            .spawn(move || {
                while let Ok(request) = receiver.recv_blocking() {
                    request.responder.respond(resolve_request(
                        &cache_root,
                        &payload_root,
                        &worker_packages
                            .read()
                            .unwrap_or_else(|error| error.into_inner()),
                        &request.webview_label,
                        &request.request,
                    ));
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            requests,
            packages,
            thread: std::sync::Mutex::new(Some(thread)),
        })
    }

    pub(crate) fn set_packages(&self, packages: std::collections::HashMap<String, PathBuf>) {
        *self
            .packages
            .write()
            .unwrap_or_else(|error| error.into_inner()) = packages;
    }

    pub(crate) fn shutdown(&self) {
        self.requests.close();
        if let Some(thread) = self
            .thread
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && thread.join().is_err()
        {
            tracing::error!("icon protocol worker panicked");
        }
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
        // Reject overload here; per-request async tasks would bypass the queue bound.
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

impl Drop for IconProtocol {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn resolve_request(
    cache_root: &std::path::Path,
    payload_root: &std::path::Path,
    packages: &std::collections::HashMap<String, PathBuf>,
    webview_label: &str,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if !matches!(webview_label, "launcher" | "settings") || request.method() != Method::GET {
        return response(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    }
    let segments = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    let [extension_id, source, rest @ ..] = segments.as_slice() else {
        return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new());
    };
    if !nanika_foundation::is_valid_extension_id(extension_id) {
        return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new());
    }
    if webview_label == "settings" && *source != "package" {
        return response(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    }
    let (root, relative) = match (*source, rest) {
        ("package", segments) => {
            let relative = segments.join("/");
            if !nanika_protocol::is_valid_package_icon_path(&relative) {
                return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new());
            }
            let Some(root) = packages.get(*extension_id) else {
                return response(StatusCode::NOT_FOUND, "text/plain", Vec::new());
            };
            (root.clone(), relative)
        }
        ("payload", [file]) if nanika_protocol::is_valid_resource_path(file) => {
            (payload_root.join(extension_id), (*file).to_owned())
        }
        ("cache", [key, file])
            if nanika_protocol::IconReference::new(*key).is_ok()
                && matches!(*file, "32.png" | "64.png" | "128.png" | "512.png") =>
        {
            (
                cache_root.join("icons").join(extension_id),
                format!("{key}/{file}"),
            )
        }
        _ => return response(StatusCode::BAD_REQUEST, "text/plain", Vec::new()),
    };
    let mut result = match nanika_platform::read_png_resource(&root.join(relative), &root) {
        Ok(bytes) => response(StatusCode::OK, "image/png", bytes),
        Err(error) => {
            use nanika_platform::PngResourceError as Error;
            let status = match error {
                Error::NotFound => StatusCode::NOT_FOUND,
                Error::OutsideRoot => StatusCode::FORBIDDEN,
                Error::EncodedSize => StatusCode::PAYLOAD_TOO_LARGE,
                Error::Dimensions { .. } | Error::Decode(_) => StatusCode::UNPROCESSABLE_ENTITY,
                Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };
            response(status, "text/plain", Vec::new())
        }
    };
    // Package paths can be replaced by an extension update; only content-addressed artifacts are immutable.
    if *source == "package" {
        result.headers_mut().insert(
            "Cache-Control",
            tauri::http::HeaderValue::from_static("no-store"),
        );
    }
    result
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
