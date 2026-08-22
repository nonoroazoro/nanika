//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

#[path = "Candidate.rs"]
mod candidate;
mod constants;
#[path = "FrameError.rs"]
mod frame_error;
mod framing;
#[path = "Message.rs"]
mod message;

pub use candidate::*;
pub use constants::*;
pub use frame_error::*;
pub use framing::*;
pub use message::*;
