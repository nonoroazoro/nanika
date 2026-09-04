use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ExtensionPackageError;

const TRANSACTION_FILE: &str = ".package-transaction.json";

/// Durable recovery record for one destructive package artifact mutation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PackageTransaction {
    pub(crate) operation: String,
    pub(crate) extension_id: String,
    pub(crate) version: Option<String>,
    pub(crate) backup_name: String,
}

impl PackageTransaction {
    pub(crate) fn replacement(extension_id: &str, version: &str, backup_name: &str) -> Self {
        Self {
            operation: "replace".to_owned(),
            extension_id: extension_id.to_owned(),
            version: Some(version.to_owned()),
            backup_name: backup_name.to_owned(),
        }
    }

    pub(crate) fn removal(extension_id: &str, backup_name: &str) -> Self {
        Self {
            operation: "remove".to_owned(),
            extension_id: extension_id.to_owned(),
            version: None,
            backup_name: backup_name.to_owned(),
        }
    }

    pub(crate) fn load(root: &Path) -> Result<Option<Self>, ExtensionPackageError> {
        let path = root.join(TRANSACTION_FILE);
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|error| {
                ExtensionPackageError::Manifest(format!(
                    "package recovery journal is invalid: {error}"
                ))
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn save(&self, root: &Path) -> Result<(), ExtensionPackageError> {
        let path = root.join(TRANSACTION_FILE);
        if path.exists() {
            return Err(ExtensionPackageError::Manifest(
                "an incomplete package transaction requires recovery".to_owned(),
            ));
        }
        let temporary = root.join(format!(".package-transaction-{}.partial", Uuid::new_v4()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            let bytes = serde_json::to_vec(self).map_err(|error| {
                ExtensionPackageError::Manifest(format!(
                    "package recovery journal could not be encoded: {error}"
                ))
            })?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub(crate) fn clear(root: &Path) -> Result<(), ExtensionPackageError> {
        match fs::remove_file(root.join(TRANSACTION_FILE)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ExtensionPackageError> {
        if !nanika_storage::is_valid_extension_id(&self.extension_id) {
            return Err(ExtensionPackageError::Manifest(
                "package recovery journal has an invalid extension id".to_owned(),
            ));
        }
        let mut backup_components = Path::new(&self.backup_name).components();
        if !matches!(backup_components.next(), Some(Component::Normal(_)))
            || backup_components.next().is_some()
        {
            return Err(ExtensionPackageError::Manifest(
                "package recovery journal has an invalid backup name".to_owned(),
            ));
        }
        match self.operation.as_str() {
            "replace" => {
                let version = self.version.as_deref().ok_or_else(|| {
                    ExtensionPackageError::Manifest(
                        "replacement recovery journal has no version".to_owned(),
                    )
                })?;
                semver::Version::parse(version)
                    .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))?;
            }
            "remove" if self.version.is_none() => {}
            "remove" => {
                return Err(ExtensionPackageError::Manifest(
                    "removal recovery journal unexpectedly has a version".to_owned(),
                ));
            }
            _ => {
                return Err(ExtensionPackageError::Manifest(
                    "package recovery journal has an invalid operation".to_owned(),
                ));
            }
        }
        Ok(())
    }
}
