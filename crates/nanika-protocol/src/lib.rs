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
#[path = "SettingColumn.rs"]
mod setting_column;
#[path = "SettingColumnControl.rs"]
mod setting_column_control;
#[path = "SettingControl.rs"]
mod setting_control;
#[path = "SettingField.rs"]
mod setting_field;
#[path = "SettingUpdate.rs"]
mod setting_update;
#[path = "SettingValue.rs"]
mod setting_value;
#[path = "SettingsContribution.rs"]
mod settings_contribution;

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
pub use setting_column::*;
pub use setting_column_control::*;
pub use setting_control::*;
pub use setting_field::*;
pub use setting_update::*;
pub use setting_value::*;
pub use settings_contribution::*;
