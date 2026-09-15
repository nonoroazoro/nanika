use serde::{Deserialize, Serialize};

/// Declares that an extension runtime contributes dynamic Root Search results.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RootSearchContribution {}
