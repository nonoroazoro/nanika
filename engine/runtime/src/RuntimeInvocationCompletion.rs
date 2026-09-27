use crate::ExtensionInvocationOutcome;

/// Extension completion and the instance authority owning its navigation resources.
#[derive(Debug)]
pub struct RuntimeInvocationCompletion {
    pub instance_id: u64,
    pub outcome: ExtensionInvocationOutcome,
}
