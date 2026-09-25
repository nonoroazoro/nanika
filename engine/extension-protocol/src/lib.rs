//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

#[path = "Action.rs"]
mod action;
#[path = "ActionInvocation.rs"]
mod action_invocation;
#[path = "ActionStyle.rs"]
mod action_style;
#[path = "Candidate.rs"]
mod candidate;
#[path = "CandidateKind.rs"]
mod candidate_kind;
#[path = "ClipboardContent.rs"]
mod clipboard_content;
mod constants;
#[path = "ContributionIcon.rs"]
mod contribution_icon;
#[path = "DetailContent.rs"]
mod detail_content;
#[path = "DetailView.rs"]
mod detail_view;
#[path = "ExtensionConfiguration.rs"]
mod extension_configuration;
#[path = "FrameError.rs"]
mod frame_error;
mod framing;
#[path = "HostServiceRequest.rs"]
mod host_service_request;
#[path = "HostServiceResponse.rs"]
mod host_service_response;
#[path = "IconReference.rs"]
mod icon_reference;
#[path = "ImageSource.rs"]
mod image_source;
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
#[path = "View.rs"]
mod view;
#[path = "ViewEvent.rs"]
mod view_event;
#[path = "ViewFile.rs"]
mod view_file;
#[path = "ViewFilter.rs"]
mod view_filter;
#[path = "ViewFilterOption.rs"]
mod view_filter_option;
#[path = "ViewItemIcon.rs"]
mod view_item_icon;
pub use view_file::ViewFile;
#[path = "ViewMetadata.rs"]
mod view_metadata;

pub use action::*;
pub use action_invocation::*;
pub use action_style::*;
pub use candidate::*;
pub use candidate_kind::*;
pub use clipboard_content::*;
pub use constants::*;
pub use contribution_icon::*;
pub use detail_content::*;
pub use detail_view::*;
pub use extension_configuration::*;
pub use frame_error::*;
pub use framing::*;
pub use host_service_request::*;
pub use host_service_response::*;
pub use icon_reference::*;
pub use image_source::*;
pub use launch_arguments::*;
pub use launch_descriptor::*;
pub use list_item::*;
pub use list_layout::*;
pub use list_section::*;
pub use list_view::*;
pub use message::*;
pub use navigation_effect::*;
pub use view::*;
pub use view_event::*;
pub use view_filter::*;
pub use view_filter_option::*;
pub use view_item_icon::*;
pub use view_metadata::*;

#[path = "OperationProgress.rs"]
mod operation_progress;
pub use operation_progress::*;
