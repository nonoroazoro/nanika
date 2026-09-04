use nanika_protocol::NavigationEffect;

/// Public completion state for one extension invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInvocationCompletion {
    pub effect: Option<NavigationEffect>,
    pub has_output: bool,
    pub cancelled: bool,
}
