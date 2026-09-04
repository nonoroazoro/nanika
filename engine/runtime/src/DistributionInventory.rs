use std::collections::HashSet;

use serde::Deserialize;

use crate::DistributionExtension;

/// Version-controlled inventory for the built-in extension distribution.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DistributionInventory {
    pub extensions: Vec<DistributionExtension>,
}

impl DistributionInventory {
    pub fn parse(source: &str) -> Result<Self, String> {
        let inventory: Self = serde_json::from_str(source).map_err(|error| error.to_string())?;
        inventory.validate()?;
        Ok(inventory)
    }

    fn validate(&self) -> Result<(), String> {
        let mut identifiers = HashSet::with_capacity(self.extensions.len());
        for extension in &self.extensions {
            if !nanika_core::BUILTIN_EXTENSION_IDS.contains(&extension.id.as_str()) {
                return Err(format!(
                    "distribution contains an unreserved built-in extension id: {}",
                    extension.id
                ));
            }
            if !identifiers.insert(extension.id.as_str()) {
                return Err(format!(
                    "distribution contains a duplicate extension id: {}",
                    extension.id
                ));
            }
            if extension.binary_name.is_empty()
                || extension.binary_name.contains('/')
                || extension.binary_name.contains('\\')
            {
                return Err(format!(
                    "distribution contains an invalid extension binary name: {}",
                    extension.binary_name
                ));
            }
            extension.runtime.validate()?;
        }
        Ok(())
    }
}
