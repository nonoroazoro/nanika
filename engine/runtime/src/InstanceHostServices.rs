use crate::{ExtensionInstance, HostServiceHandler};
use nanika_protocol::{HostServiceRequest, HostServiceResponse};
use std::sync::{Arc, mpsc::Receiver};

pub(crate) struct InstanceHostServices {
    pub(crate) instance: Arc<ExtensionInstance>,
    pub(crate) services: Arc<dyn HostServiceHandler>,
}

impl HostServiceHandler for InstanceHostServices {
    fn submit(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
    ) -> Result<Receiver<Result<HostServiceResponse, String>>, String> {
        // Admission precedes the potentially blocking native queue. The owning
        // invocation retains this accepted request through its terminal result.
        if !self.instance.is_active() {
            return Err("The extension instance has been retired.".into());
        }
        self.services.submit(extension_id, request)
    }
}
