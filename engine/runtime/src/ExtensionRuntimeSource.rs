use crate::ExtensionRuntime;
use nanika_extension_package::ExtensionActivation;
use nanika_protocol::ExtensionConfiguration;

/// Process ownership is transferred to the worker, including creation and initialization.
pub enum ExtensionRuntimeSource {
    Started(Box<ExtensionRuntime>),
    Factory {
        activation: ExtensionActivation,
        live_configuration: bool,
        start: Box<dyn FnOnce(ExtensionConfiguration) -> std::io::Result<ExtensionRuntime> + Send>,
    },
}

impl ExtensionRuntimeSource {
    pub(crate) fn is_deferred(&self) -> bool {
        matches!(
            self,
            Self::Factory {
                activation: ExtensionActivation::OnDemand,
                ..
            }
        )
    }

    pub(crate) fn supports_live_configuration(&self) -> bool {
        match self {
            Self::Started(runtime) => runtime.supports_live_configuration(),
            Self::Factory {
                live_configuration, ..
            } => *live_configuration,
        }
    }

    pub(crate) fn start(
        self,
        configuration: ExtensionConfiguration,
    ) -> std::io::Result<ExtensionRuntime> {
        match self {
            Self::Started(runtime) => Ok(*runtime),
            Self::Factory { start, .. } => start(configuration),
        }
    }
}
