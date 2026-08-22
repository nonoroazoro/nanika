use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use nanika_host::HostServiceHandler;
use nanika_protocol::{HostServiceRequest, HostServiceResponse};

pub struct TestHostServices {
    requests: Mutex<Vec<(String, HostServiceRequest)>>,
}

impl TestHostServices {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn request_count(&self) -> usize {
        self.requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }
}

impl HostServiceHandler for TestHostServices {
    fn submit(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
        _deadline: Instant,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        self.requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push((extension_id.to_owned(), request));
        let (response, result) = mpsc::sync_channel(1);
        let _ = response.send(Ok(HostServiceResponse::Launched));
        Ok(result)
    }
}
