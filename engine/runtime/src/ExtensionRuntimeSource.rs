use crate::ExtensionRuntime;

/// A dormant process has no pipes, readers or native process tree until activation.
pub enum ExtensionRuntimeSource {
    Started(Box<ExtensionRuntime>),
    OnDemand(Box<dyn FnOnce() -> std::io::Result<ExtensionRuntime> + Send>),
}

impl ExtensionRuntimeSource {
    pub(crate) fn is_deferred(&self) -> bool {
        matches!(self, Self::OnDemand(_))
    }

    pub(crate) fn supports_live_configuration(&self) -> bool {
        match self {
            Self::Started(runtime) => runtime.supports_live_configuration(),
            Self::OnDemand(_) => true,
        }
    }

    pub(crate) fn start(self) -> std::io::Result<ExtensionRuntime> {
        match self {
            Self::Started(runtime) => Ok(*runtime),
            Self::OnDemand(start) => start(),
        }
    }
}
