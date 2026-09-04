use nanika_protocol::NavigationEffect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExtensionInvocationOutcome {
    Completed {
        effect: NavigationEffect,
        has_output: bool,
    },
    Cancelled,
}
