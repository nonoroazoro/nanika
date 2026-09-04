use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Instant;

use nanika_host::HostServiceHandler;
use nanika_protocol::{HostServiceRequest, HostServiceResponse};

pub struct BlockingHostServices {
    responses: Mutex<Vec<SyncSender<Result<HostServiceResponse, String>>>>,
}

impl BlockingHostServices {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
        }
    }
}

impl HostServiceHandler for BlockingHostServices {
    fn submit(
        &self,
        _extension_id: &str,
        _request: HostServiceRequest,
        _deadline: Instant,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.responses
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(response);
        Ok(result)
    }
}
