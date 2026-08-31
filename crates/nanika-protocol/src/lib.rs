//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

#[path = "Candidate.rs"]
mod candidate;
#[path = "ClipboardContent.rs"]
mod clipboard_content;
mod constants;
#[path = "DetailView.rs"]
mod detail_view;
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
#[path = "ListItem.rs"]
mod list_item;
#[path = "ListLayout.rs"]
mod list_layout;
#[path = "ListSection.rs"]
mod list_section;
#[path = "ListView.rs"]
mod list_view;
#[path = "Message.rs"]
mod message;
#[path = "NavigationEffect.rs"]
mod navigation_effect;
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
#[path = "View.rs"]
mod view;
#[path = "ViewAction.rs"]
mod view_action;
#[path = "ViewActionStyle.rs"]
mod view_action_style;
#[path = "ViewEvent.rs"]
mod view_event;
#[path = "ViewFilter.rs"]
mod view_filter;
#[path = "ViewFilterOption.rs"]
mod view_filter_option;
#[path = "ViewMetadata.rs"]
mod view_metadata;

pub use candidate::*;
pub use clipboard_content::*;
pub use constants::*;
pub use detail_view::*;
pub use frame_error::*;
pub use framing::*;
pub use host_service_request::*;
pub use host_service_response::*;
pub use launch_arguments::*;
pub use launch_descriptor::*;
pub use list_item::*;
pub use list_layout::*;
pub use list_section::*;
pub use list_view::*;
pub use message::*;
pub use navigation_effect::*;
pub use setting_column::*;
pub use setting_column_control::*;
pub use setting_control::*;
pub use setting_field::*;
pub use setting_update::*;
pub use setting_value::*;
pub use settings_contribution::*;
pub use view::*;
pub use view_action::*;
pub use view_action_style::*;
pub use view_event::*;
pub use view_filter::*;
pub use view_filter_option::*;
pub use view_metadata::*;
