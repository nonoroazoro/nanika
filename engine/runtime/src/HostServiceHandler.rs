use crate::PreparedHostService;
use nanika_protocol::HostServiceRequest;

pub trait HostServiceHandler: Send + Sync {
    /// Prepare stable inputs before the caller makes the final admission decision.
    fn prepare(
        &self,
        extension_id: &str,
        request: HostServiceRequest,
        interruption: &mut dyn FnMut() -> crate::ExtensionInterruption
    ) -> Result<PreparedHostService<'_>, String>;
}
