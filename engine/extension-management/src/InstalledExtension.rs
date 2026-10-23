use std::path::PathBuf;
use std::sync::Arc;

use nanika_localization::ResourceSet;

use crate::{ExtensionContributions, ExtensionManifest, ExtensionPackageError, ExtensionProtocol};

/// Validated installed descriptor, independent of enablement or a live process.
#[derive(Debug, Clone)]
pub struct InstalledExtension {
    pub extension_id: String,
    pub name: nanika_protocol::LocalizedText,
    pub name_search_values: Vec<String>,
    pub configuration_search_values: std::collections::BTreeMap<String, Vec<String>>,
    pub icon: String,
    pub program: PathBuf,
    pub resource_root: PathBuf,
    pub localization: Option<Arc<ResourceSet>>,
    pub protocol: ExtensionProtocol,
    pub activation: crate::ExtensionActivation,
    pub permissions: Vec<String>,
    pub contributes: ExtensionContributions
}

impl InstalledExtension {
    /// Admit package resources before publishing the descriptor or replacing an installed package.
    /// Completeness comes from the host inventory, never from an author-supplied identity flag.
    pub fn from_manifest(
        manifest: ExtensionManifest,
        program: PathBuf,
        resource_root: PathBuf,
        require_complete: bool
    ) -> Result<Self, ExtensionPackageError> {
        let localization = manifest
            .localization
            .as_ref()
            .map(|declaration| ResourceSet::load(&resource_root, declaration, require_complete))
            .transpose()
            .map_err(|error| {
                ExtensionPackageError::Manifest(format!("{}: localization: {error}", manifest.id))
            })?
            .map(Arc::new);
        let name_search_values = Self::_name_search_values(&manifest, localization.as_deref())?;
        Self::_validate_groups(&manifest)?;
        Self::_validate_contributions(&manifest, localization.as_deref())?;
        for command in &manifest.contributes.commands {
            nanika_localization::validate_action_presentation(
                std::slice::from_ref(&command.action),
                localization.as_deref()
            )
            .map_err(|error| {
                ExtensionPackageError::Manifest(format!(
                    "{}: command {}: {error}",
                    manifest.id, command.command
                ))
            })?;
        }
        let configuration_search_values = manifest
            .contributes
            .configuration
            .as_ref()
            .map(|configuration| configuration._prepare_presentation(localization.as_deref()))
            .transpose()
            .map_err(|error| {
                ExtensionPackageError::Manifest(format!("{}: configuration: {error}", manifest.id))
            })?
            .unwrap_or_default();
        Ok(Self {
            extension_id: manifest.id,
            name: manifest.name,
            name_search_values,
            configuration_search_values,
            icon: manifest.icon,
            program,
            resource_root,
            localization,
            protocol: manifest.runtime,
            activation: manifest.activation,
            permissions: manifest.permissions,
            contributes: manifest.contributes
        })
    }

    fn _validate_contributions(
        manifest: &ExtensionManifest,
        resources: Option<&ResourceSet>
    ) -> Result<(), ExtensionPackageError> {
        let mut fields = Vec::new();
        for command in &manifest.contributes.commands {
            fields.push((&command.command, "title", &command.title, 128));
            fields.push((&command.command, "description", &command.description, 512));
            fields.extend(
                command
                    .category
                    .iter()
                    .map(|text| (&command.command, "category", text, 128))
            );
            fields.extend(
                command
                    .keywords
                    .iter()
                    .map(|text| (&command.command, "keyword", text, 64))
            );
        }
        for view in &manifest.contributes.views {
            fields.push((&view.id, "title", &view.title, 128));
            fields.push((&view.id, "description", &view.description, 512));
            fields.extend(
                view.category
                    .iter()
                    .map(|text| (&view.id, "category", text, 128))
            );
            fields.extend(
                view.keywords
                    .iter()
                    .map(|text| (&view.id, "keyword", text, 64))
            );
        }
        for (id, field, text, maximum) in fields {
            if let nanika_protocol::LocalizedText::Message(_) = text {
                let error = |detail| {
                    ExtensionPackageError::Manifest(format!(
                        "{}: contribution {id} {field}: {detail}",
                        manifest.id
                    ))
                };
                let resources = resources
                    .ok_or_else(|| error("localization resources are required".to_owned()))?;
                for locale in [
                    nanika_localization::SupportedLocale::English,
                    nanika_localization::SupportedLocale::SimplifiedChinese
                ] {
                    for platform in ["macos", "windows"] {
                        let rendered = resources
                            .format(locale, text, platform, maximum, false)
                            .map_err(error)?;
                        if rendered.chars().any(char::is_control) {
                            return Err(error("invalid control character".to_owned()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn _validate_groups(manifest: &ExtensionManifest) -> Result<(), ExtensionPackageError> {
        let mut groups: Vec<_> = manifest
            .contributes
            .commands
            .iter()
            .filter_map(|command| command.action.group.clone())
            .collect();
        if let Some(configuration) = &manifest.contributes.configuration {
            groups.extend(
                configuration
                    .properties
                    .values()
                    .filter_map(|property| property.group.clone())
            );
            groups.extend(
                configuration
                    .actions
                    .iter()
                    .filter_map(|item| item.action.group.clone())
            );
            groups.extend(configuration.quick_menu.iter().map(|group| {
                nanika_protocol::PresentationGroup {
                    id: group.id.clone(),
                    title: group.title.clone()
                }
            }));
        }
        nanika_protocol::PresentationGroup::validate_definitions(&groups)
            .map_err(|error| ExtensionPackageError::Manifest(format!("{}: {error}", manifest.id)))
    }

    fn _name_search_values(
        manifest: &ExtensionManifest,
        resources: Option<&ResourceSet>
    ) -> Result<Vec<String>, ExtensionPackageError> {
        if let nanika_protocol::LocalizedText::Literal(name) = &manifest.name {
            crate::extension_manifest::_validate_name(name)?;
            return Ok(vec![name.clone()]);
        }
        let resources = resources.ok_or_else(|| {
            ExtensionPackageError::Manifest(format!(
                "{}: name requires localization resources",
                manifest.id
            ))
        })?;
        let mut values = Vec::new();
        for locale in [
            nanika_localization::SupportedLocale::English,
            nanika_localization::SupportedLocale::SimplifiedChinese
        ] {
            for platform in ["macos", "windows"] {
                let name = resources
                    .format(locale, &manifest.name, platform, 128, false)
                    .map_err(|error| {
                        ExtensionPackageError::Manifest(format!(
                            "{}: name: {}: {platform}: {error}",
                            manifest.id,
                            locale.as_str()
                        ))
                    })?;
                crate::extension_manifest::_validate_name(&name).map_err(|error| {
                    ExtensionPackageError::Manifest(format!(
                        "{}: name: {}: {platform}: {error}",
                        manifest.id,
                        locale.as_str()
                    ))
                })?;
                if platform == nanika_platform::target_platform().as_str()
                    && !values.contains(&name)
                {
                    values.push(name);
                }
            }
        }
        Ok(values)
    }
}
