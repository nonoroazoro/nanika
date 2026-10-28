//! Shared extension protocol boundary.

#![forbid(unsafe_code)]

#[path = "ProtocolError.rs"]
mod protocol_error;
pub use protocol_error::ProtocolError;

#[path = "Action.rs"]
mod action;
#[path = "ActionIcon.rs"]
mod action_icon;
pub use action_icon::*;
#[path = "ActionInvocation.rs"]
mod action_invocation;
#[path = "ActionShortcut.rs"]
mod action_shortcut;
pub use action_shortcut::*;
#[path = "ShortcutKey.rs"]
mod shortcut_key;
pub use shortcut_key::*;
#[path = "ShortcutModifier.rs"]
mod shortcut_modifier;
pub use shortcut_modifier::*;
#[path = "ActionStyle.rs"]
mod action_style;
#[path = "Candidate.rs"]
mod candidate;
#[path = "CandidateKind.rs"]
mod candidate_kind;
#[path = "CandidateSubtitle.rs"]
mod candidate_subtitle;
#[path = "ClipboardContent.rs"]
mod clipboard_content;
mod constants;
#[path = "TextChunk.rs"]
mod text_chunk;
#[path = "TextDocument.rs"]
mod text_document;
#[path = "TextDocumentIndex.rs"]
mod text_document_index;
#[path = "TextRead.rs"]
mod text_read;
pub use text_chunk::*;
pub use text_document::*;
pub use text_document_index::*;
pub use text_read::*;
#[path = "SystemAction.rs"]
mod system_action;
pub use system_action::*;
#[path = "DetailContent.rs"]
mod detail_content;
#[path = "DetailView.rs"]
mod detail_view;
#[path = "IntegerLimit.rs"]
mod integer_limit;
pub use integer_limit::*;
#[path = "ExtensionConfiguration.rs"]
mod extension_configuration;
#[path = "FrameBuffer.rs"]
mod frame_buffer;
#[path = "FrameError.rs"]
mod frame_error;
mod framing;
#[path = "HostServiceRequest.rs"]
mod host_service_request;
#[path = "HostServiceResponse.rs"]
mod host_service_response;
#[path = "IconReference.rs"]
mod icon_reference;
#[path = "IconSource.rs"]
mod icon_source;
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
pub use candidate_subtitle::*;
pub use clipboard_content::*;
pub use constants::*;
pub use detail_content::*;
pub use detail_view::*;
pub use extension_configuration::*;
pub use frame_error::*;
pub use framing::*;
pub use host_service_request::*;
pub use host_service_response::*;
#[path = "HostServiceFailure.rs"]
mod host_service_failure;
pub use host_service_failure::HostServiceFailure;
#[path = "HostServiceFailureKind.rs"]
mod host_service_failure_kind;
pub use host_service_failure_kind::HostServiceFailureKind;
pub use icon_reference::*;
pub use icon_source::*;
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

#[path = "CandidateUpdate.rs"]
mod candidate_update;
pub use candidate_update::CandidateUpdate;

#[path = "CatalogBatch.rs"]
mod catalog_batch;
pub use catalog_batch::CatalogBatch;
#[path = "CatalogPublisher.rs"]
mod catalog_publisher;
pub use catalog_publisher::CatalogPublisher;
#[path = "CatalogPublication.rs"]
mod catalog_publication;
use catalog_publication::CatalogPublication;

#[path = "ListSelection.rs"]
mod list_selection;
pub use list_selection::*;

#[path = "TextInvalidation.rs"]
mod text_invalidation;
pub use text_invalidation::TextInvalidation;

/// Reserved Invoke entry for manifest-declared Settings actions.
pub const SETTINGS_ENTRY_ID: &str = "nanika.settings";

#[path = "TranslatedText.rs"]
mod translated_text;
pub use translated_text::{TranslatedText, canonical_language_tag};
#[path = "LocalizedText.rs"]
mod localized_text;
pub use localized_text::*;
#[path = "MessageReference.rs"]
mod message_reference;
pub use message_reference::*;
#[path = "MessageArgument.rs"]
mod message_argument;
pub use message_argument::*;
#[path = "MessageArguments.rs"]
mod message_arguments;
pub use message_arguments::*;

#[path = "PresentationGroup.rs"]
mod presentation_group;
pub use presentation_group::PresentationGroup;

#[path = "ViewMetadataValue.rs"]
mod view_metadata_value;
pub use view_metadata_value::ViewMetadataValue;
