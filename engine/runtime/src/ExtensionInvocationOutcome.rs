use nanika_protocol::NavigationEffect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionInvocationOutcome {
    Completed {
        effect: NavigationEffect,
        has_output: bool,
    },
    Cancelled,
}
