use crate::ExtensionInvocationOutcome;

/// Recording can fail after the extension has already completed its side effect.
/// Keep the outcome available so its navigation resources always retain an owner.
#[derive(Debug)]
pub struct RuntimeInvocationCompletion {
    pub instance_id: u64,
    pub outcome: ExtensionInvocationOutcome,
    pub recording_error: Option<String>,
}
