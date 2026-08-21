//! Shared host types with no domain capabilities.

#![forbid(unsafe_code)]

/// Product display name.
pub const PRODUCT_NAME: &str = "Nanika";

/// Current project identity used by platform adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectIdentity {
    pub qualifier: &'static str,
    pub organization: &'static str,
    pub application: &'static str,
    pub bundle_id: &'static str,
}

/// Current pre-1.0 identity baseline.
pub const PROJECT_IDENTITY: ProjectIdentity = ProjectIdentity {
    qualifier: "com",
    organization: "nanika",
    application: "nanika",
    bundle_id: "com.nanika.nanika",
};
