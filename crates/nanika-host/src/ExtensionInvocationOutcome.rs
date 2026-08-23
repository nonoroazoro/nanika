#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExtensionInvocationOutcome {
    Completed { has_output: bool },
    Cancelled,
}
