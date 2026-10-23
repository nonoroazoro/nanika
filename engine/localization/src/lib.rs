//! Immutable Fluent resources and the shared UI language policy. No platform or shell dependency.
#![forbid(unsafe_code)]

#[path = "SupportedLocale.rs"]
mod supported_locale;
pub use supported_locale::SupportedLocale;
#[path = "LanguagePreference.rs"]
mod language_preference;
pub use language_preference::LanguagePreference;
#[path = "LocalizationSnapshot.rs"]
mod localization_snapshot;
pub use localization_snapshot::LocalizationSnapshot;
#[path = "LocalizationResources.rs"]
mod localization_resources;
pub use localization_resources::LocalizationResources;
#[path = "MessageContract.rs"]
mod message_contract;
pub use message_contract::MessageContract;
#[path = "LocaleBundle.rs"]
mod locale_bundle;
pub use locale_bundle::LocaleBundle;
#[path = "BoundedOutput.rs"]
mod bounded_output;
mod constants;
#[path = "ResourceValidator.rs"]
mod resource_validator;
pub use constants::*;

mod actions;
mod candidates;
mod progress;
mod views;
pub use candidates::candidate_search_text;
pub use progress::validate_progress_presentation;
pub use views::{validate_navigation_presentation, validate_view_presentation};
mod source;
mod typescript;
pub use actions::validate_action_presentation;
pub use typescript::typescript_message_type;

#[path = "PreparedLocale.rs"]
mod prepared_locale;
pub use prepared_locale::PreparedLocale;

#[path = "PreparedResources.rs"]
mod prepared_resources;
pub use prepared_resources::PreparedResources;

#[path = "ResourceSet.rs"]
mod resource_set;
pub use resource_set::ResourceSet;

#[path = "PreparedError.rs"]
mod prepared_error;
pub use prepared_error::PreparedError;
