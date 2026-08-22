//! Shared host types with no domain capabilities.

#![forbid(unsafe_code)]

mod constants;
#[path = "ProjectIdentity.rs"]
mod project_identity;

pub use constants::*;
pub use project_identity::*;
