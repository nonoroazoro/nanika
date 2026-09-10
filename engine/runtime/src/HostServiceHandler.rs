use std::sync::mpsc::Receiver;

use nanika_protocol::{HostServiceRequest, HostServiceResponse};

/// Common host service boundary used by built-in and external extensions.
pub trait HostServiceHandler: Send + Sync {
    fn submit(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String>;
}
