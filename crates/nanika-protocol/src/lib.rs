//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

#[path = "Candidate.rs"]
mod candidate;
#[path = "ClipboardContent.rs"]
mod clipboard_content;
mod constants;
#[path = "FrameError.rs"]
mod frame_error;
mod framing;
#[path = "HostServiceRequest.rs"]
mod host_service_request;
#[path = "HostServiceResponse.rs"]
mod host_service_response;
#[path = "LaunchArguments.rs"]
mod launch_arguments;
#[path = "LaunchDescriptor.rs"]
mod launch_descriptor;
#[path = "Message.rs"]
mod message;

pub use candidate::*;
pub use clipboard_content::*;
pub use constants::*;
pub use frame_error::*;
pub use framing::*;
pub use host_service_request::*;
pub use host_service_response::*;
pub use launch_arguments::*;
pub use launch_descriptor::*;
pub use message::*;
