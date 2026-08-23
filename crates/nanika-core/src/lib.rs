//! Shared host types with no domain capabilities.

#![forbid(unsafe_code)]

mod constants;
mod extension_id;
#[path = "ProjectIdentity.rs"]
mod project_identity;

pub use constants::*;
pub use extension_id::*;
pub use project_identity::*;
