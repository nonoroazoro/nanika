//! Validation and atomic installation for external extension packages.

#![forbid(unsafe_code)]

#[path = "ActiveExtension.rs"]
mod active_extension;
#[path = "CommandContribution.rs"]
mod command_contribution;
#[path = "CommandIcon.rs"]
mod command_icon;
#[path = "ConfigurationContribution.rs"]
mod configuration_contribution;
#[path = "ConfigurationProperty.rs"]
mod configuration_property;
#[path = "ConfigurationSchema.rs"]
mod configuration_schema;
#[path = "ExtensionContributions.rs"]
mod extension_contributions;
#[path = "ExtensionManifest.rs"]
mod extension_manifest;
#[path = "ExtensionPackageError.rs"]
mod extension_package_error;
#[path = "ExtensionProtocol.rs"]
mod extension_protocol;
#[path = "ExtensionResolutionError.rs"]
mod extension_resolution_error;
#[path = "ExtensionTarget.rs"]
mod extension_target;
mod package;
#[path = "PackageOperation.rs"]
mod package_operation;
#[path = "PackageTransaction.rs"]
mod package_transaction;
#[path = "RootSearchContribution.rs"]
mod root_search_contribution;
#[path = "StagedPackage.rs"]
mod staged_package;
#[path = "StagingDirectory.rs"]
mod staging_directory;

pub use active_extension::*;
pub use command_contribution::*;
pub use command_icon::*;
pub use configuration_contribution::*;
pub use configuration_property::*;
pub use configuration_schema::*;
pub use extension_contributions::*;
pub use extension_manifest::*;
pub use extension_package_error::*;
pub use extension_protocol::*;
pub use extension_resolution_error::*;
pub use extension_target::*;
pub use package::*;
pub(crate) use package_operation::*;
pub(crate) use package_transaction::*;
pub use root_search_contribution::*;
pub(crate) use staged_package::*;
pub(crate) use staging_directory::*;
