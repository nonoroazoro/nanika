/// Current project identity used by platform adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectIdentity {
    pub bundle_id: &'static str,
}
