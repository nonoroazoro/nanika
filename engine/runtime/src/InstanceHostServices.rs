use crate::{ExtensionInstance, HostServiceHandler};
use nanika_protocol::HostServiceRequest;
use std::sync::Arc;

pub(crate) struct InstanceHostServices {
    pub(crate) instance: Arc<ExtensionInstance>,
    pub(crate) services: Arc<dyn HostServiceHandler>
}

impl HostServiceHandler for InstanceHostServices {
    fn prepare(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
        interruption: &mut dyn FnMut() -> crate::ExtensionInterruption
    ) -> Result<crate::PreparedHostService<'_>, String> {
        if !self.instance.is_active() {
            return Err("The extension instance has been retired.".into());
        }
        let prepared = self.services.prepare(extension_id, request, interruption)?;
        Ok(crate::PreparedHostService::new(move || {
            // This short check is the admission point. Retirement after it must
            // preserve the prepared input and eventual native result.
            if !self.instance.is_active() {
                return Err("The extension instance has been retired.".into());
            }
            prepared.admit()
        }))
    }
}
