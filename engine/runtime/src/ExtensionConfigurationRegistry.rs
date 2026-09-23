use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use nanika_config::{ConfigStore, ExtensionConfigurationFile};
use nanika_extension_package::ConfigurationContribution;
use nanika_protocol::ExtensionConfiguration;
use serde_json::Value;

use crate::RuntimeExtensionConfiguration;

#[derive(Debug, Clone)]
struct RegisteredConfiguration {
    contribution: ConfigurationContribution,
    values: BTreeMap<String, Value>,
}

/// Host-owned configuration schema and persisted values for active extensions.
pub(crate) struct ExtensionConfigurationRegistry {
    store: ConfigStore,
    registered: Mutex<HashMap<String, RegisteredConfiguration>>,
}

impl ExtensionConfigurationRegistry {
    pub(crate) fn new(store: ConfigStore) -> Self {
        Self {
            store,
            registered: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn register(
        &self,
        extension_id: &str,
        contribution: Option<&ConfigurationContribution>,
    ) -> Result<ExtensionConfiguration, String> {
        let Some(contribution) = contribution else {
            return Ok(ExtensionConfiguration::default());
        };
        contribution
            .validate()
            .map_err(|error| format!("invalid configuration schema: {error}"))?;
        let path = self.store.extension_configuration_file(extension_id);
        let values = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let stored = self
                    .store
                    .load::<ExtensionConfigurationFile>(&path)
                    .map_err(|error| error.to_string())?;
                stored.validate_format()?;
                let mut effective = contribution.defaults();
                effective.extend(stored.values);
                contribution.validate_values(&effective)?;
                effective
            }
            Ok(_) => {
                return Err(format!(
                    "extension configuration path is not a file: {}",
                    path.display()
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => contribution.defaults(),
            Err(error) => return Err(error.to_string()),
        };
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if registered
            .insert(
                extension_id.to_owned(),
                RegisteredConfiguration {
                    contribution: contribution.clone(),
                    values: values.clone(),
                },
            )
            .is_some()
        {
            return Err(format!(
                "extension configuration is already registered: {extension_id}"
            ));
        }
        Ok(ExtensionConfiguration::new(values))
    }

    pub(crate) fn update(
        &self,
        extension_id: &str,
        values: BTreeMap<String, Value>,
    ) -> Result<ExtensionConfiguration, String> {
        // Serialize validation, persistence, and the in-memory snapshot so concurrent
        // Settings requests cannot leave disk and memory describing different updates.
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (contribution, previous_values) = registered
            .get(extension_id)
            .map(|registered| (registered.contribution.clone(), registered.values.clone()))
            .ok_or_else(|| {
                format!("extension does not contribute configuration: {extension_id}")
            })?;
        // Settings submits every visible field. Only fields hidden on this platform
        // may be omitted, so an incomplete form cannot silently reuse stale values.
        let visible = contribution.for_platform(nanika_platform::target_platform());
        for key in visible.properties.keys() {
            if !values.contains_key(key) {
                return Err(format!("missing configuration property: {key}"));
            }
        }
        let mut effective = previous_values;
        effective.extend(values);
        contribution.validate_values(&effective)?;

        let path = self.store.extension_configuration_file(extension_id);
        if path.is_file() {
            let updates = effective
                .iter()
                .map(|(key, value)| (key.clone(), Some(value.clone())))
                .collect::<Vec<_>>();
            self.store
                .update_object::<ExtensionConfigurationFile>(&path, "values", updates, |stored| {
                    stored.validate_format()?;
                    contribution.validate_values(&stored.values)
                })
                .map_err(|error| error.to_string())?;
        } else {
            self.store
                .save(&path, &ExtensionConfigurationFile::new(effective.clone()))
                .map_err(|error| error.to_string())?;
        }

        let current = registered
            .get_mut(extension_id)
            .ok_or_else(|| format!("extension configuration disappeared: {extension_id}"))?;
        current.values = effective.clone();
        Ok(ExtensionConfiguration::new(effective))
    }

    pub(crate) fn snapshots(&self) -> Vec<RuntimeExtensionConfiguration> {
        let registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let platform = nanika_platform::target_platform();
        let mut snapshots = registered
            .iter()
            .map(|(extension_id, registered)| {
                let contribution = registered.contribution.for_platform(platform);
                let values = registered
                    .values
                    .iter()
                    .filter(|(key, _)| contribution.properties.contains_key(*key))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();
                RuntimeExtensionConfiguration {
                    extension_id: extension_id.clone(),
                    contribution,
                    values,
                }
            })
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.extension_id.cmp(&right.extension_id));
        snapshots
    }
}
