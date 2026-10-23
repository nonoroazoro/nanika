use crate::ExtensionRuntime;
use nanika_extension_package::ExtensionActivation;
use nanika_protocol::ExtensionConfiguration;

/// Process ownership is transferred to the worker, including creation and initialization.
pub struct ExtensionRuntimeSource {
    pub localization: Option<std::sync::Arc<nanika_localization::ResourceSet>>,
    pub activation: ExtensionActivation,
    pub live_configuration: bool,
    pub start: Box<dyn FnOnce(ExtensionConfiguration) -> std::io::Result<ExtensionRuntime> + Send>
}

impl ExtensionRuntimeSource {
    pub(crate) fn is_deferred(&self) -> bool {
        self.activation == ExtensionActivation::OnDemand
    }

    pub(crate) fn supports_live_configuration(&self) -> bool {
        self.live_configuration
    }

    pub(crate) fn start(
        self,
        configuration: ExtensionConfiguration
    ) -> std::io::Result<ExtensionRuntime> {
        (self.start)(configuration)
    }
}
