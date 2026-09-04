/// Current project identity used by platform adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectIdentity {
    pub qualifier: &'static str,
    pub organization: &'static str,
    pub application: &'static str,
    pub bundle_id: &'static str,
}
