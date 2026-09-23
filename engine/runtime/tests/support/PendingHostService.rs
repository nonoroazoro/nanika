use nanika_host::HostServiceHandler;
use nanika_protocol::{HostServiceRequest, HostServiceResponse};
use std::sync::{Mutex, mpsc};

#[derive(Default)]
pub struct PendingHostService {
    response: Mutex<Option<mpsc::SyncSender<Result<HostServiceResponse, String>>>>,
}
impl PendingHostService {
    pub fn submitted(&self) -> bool {
        self.response.lock().unwrap().is_some()
    }
    pub fn complete(&self) -> bool {
        self.response
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(Ok(HostServiceResponse::Launched))
            .is_ok()
    }
}
impl HostServiceHandler for PendingHostService {
    fn submit(
        &self,
        _: &str,
        _: HostServiceRequest,
    ) -> Result<mpsc::Receiver<Result<HostServiceResponse, String>>, String> {
        let (response, receiver) = mpsc::sync_channel(1);
        *self.response.lock().unwrap() = Some(response);
        Ok(receiver)
    }
}
