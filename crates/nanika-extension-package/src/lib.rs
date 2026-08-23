//! Validation and atomic installation for external extension packages.

#![forbid(unsafe_code)]

#[path = "ActiveExtension.rs"]
mod active_extension;
#[path = "ExtensionManifest.rs"]
mod extension_manifest;
#[path = "ExtensionPackageError.rs"]
mod extension_package_error;
#[path = "ExtensionTarget.rs"]
mod extension_target;
mod package;
#[path = "StagedPackage.rs"]
mod staged_package;
#[path = "StagingDirectory.rs"]
mod staging_directory;

pub use active_extension::*;
pub use extension_manifest::*;
pub use extension_package_error::*;
pub use extension_target::*;
pub use package::*;
pub(crate) use staged_package::*;
pub(crate) use staging_directory::*;
