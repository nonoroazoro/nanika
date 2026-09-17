use std::collections::HashSet;
use std::path::Path;

use nanika_extension_package::parse_extension_manifest;

use crate::BuiltInExtension;

/// Host-owned selection of built-ins, each described by the ordinary extension manifest.
#[derive(Debug, Clone)]
pub struct BuiltInExtensionInventory {
    pub extensions: Vec<BuiltInExtension>,
}

impl BuiltInExtensionInventory {
    pub fn parse(sources: &[&str]) -> Result<Self, String> {
        let mut identifiers = HashSet::with_capacity(sources.len());
        let mut extensions = Vec::with_capacity(sources.len());
        for source in sources {
            let manifest = parse_extension_manifest(source).map_err(|error| error.to_string())?;
            if !nanika_foundation::BUILTIN_EXTENSION_IDS.contains(&manifest.id.as_str()) {
                return Err(format!(
                    "built-in inventory contains an unreserved extension id: {}",
                    manifest.id
                ));
            }
            if !identifiers.insert(manifest.id.clone()) {
                return Err(format!(
                    "built-in inventory contains a duplicate extension id: {}",
                    manifest.id
                ));
            }
            let target = nanika_platform::target_triple();
            let entrypoint = manifest.targets.get(target).ok_or_else(|| {
                format!(
                    "built-in extension {} does not support {target}",
                    manifest.id
                )
            })?;
            let file_name = Path::new(&entrypoint.entrypoint)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    format!(
                        "built-in extension {} has an invalid entrypoint",
                        manifest.id
                    )
                })?;
            let binary_name = file_name
                .strip_suffix(".exe")
                .unwrap_or(file_name)
                .to_owned();
            extensions.push(BuiltInExtension {
                manifest,
                binary_name,
            });
        }
        Ok(Self { extensions })
    }
}
