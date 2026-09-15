//! Shared host types with no domain capabilities.

#![forbid(unsafe_code)]

mod constants;
#[path = "DiagnosticCategory.rs"]
mod diagnostic_category;
#[path = "DiagnosticCode.rs"]
mod diagnostic_code;
mod extension_id;
#[path = "ProjectIdentity.rs"]
mod project_identity;

pub use constants::*;
pub use diagnostic_category::*;
pub use diagnostic_code::*;
pub use extension_id::*;
pub use project_identity::*;
