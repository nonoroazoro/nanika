use std::collections::{BTreeMap, HashMap};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use nanika_config::{ConfigStore, ExtensionConfigurationFile};
use nanika_extension_package::{ConfigurationContribution, ConfigurationPersistence};
use nanika_protocol::ExtensionConfiguration;
use serde_json::Value;

use crate::{ConfigurationOperation, ConfigurationSaveOutcome, RuntimeExtensionConfiguration};

#[derive(Debug, Clone)]
struct RegisteredConfiguration {
    revision: u64,
    contribution: ConfigurationContribution,
    values: BTreeMap<String, Value>,
    effective: Option<BTreeMap<String, Value>>,
}

pub(crate) struct ExtensionConfigurationRegistry {
    _revision: AtomicU64,
    store: ConfigStore,
    registered: Mutex<HashMap<String, RegisteredConfiguration>>,
    pub(crate) operations: Arc<crate::ExtensionOperationGate>,
}

impl ExtensionConfigurationRegistry {
    pub(crate) fn new(store: ConfigStore) -> Self {
        Self {
            _revision: AtomicU64::new(0),
            store,
            registered: Mutex::new(HashMap::new()),
            operations: Arc::new(crate::ExtensionOperationGate::default()),
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
        match registered.entry(extension_id.to_owned()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(RegisteredConfiguration {
                    revision: self._revision.fetch_add(1, Ordering::Release) + 1,
                    contribution: contribution.clone(),
                    values: values.clone(),
                    effective: None,
                });
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(format!(
                    "extension configuration is already registered: {extension_id}"
                ));
            }
        }
        Ok(ExtensionConfiguration::new(values))
    }

    pub(crate) fn prepare(
        self: &Arc<Self>,
        extension_id: &str,
        key: String,
        value: Value,
    ) -> Result<ConfigurationOperation, String> {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let reservation = self.operations.reserve(extension_id)?;
        let current = registered.get_mut(extension_id).ok_or_else(|| {
            format!("extension does not contribute configuration: {extension_id}")
        })?;
        let visible = current
            .contribution
            .for_platform(nanika_platform::target_platform());
        let property = visible
            .properties
            .get(&key)
            .ok_or_else(|| format!("unknown or unavailable configuration property: {key}"))?;
        let persistence = property.persistence;
        let mut values = Self::_values(current);
        values.insert(key.clone(), value.clone());
        current.contribution.validate_values(&values)?;
        Ok(ConfigurationOperation {
            _reservation: reservation,
            registry: Arc::clone(self),
            extension_id: extension_id.to_owned(),
            key,
            value,
            persistence,
            configuration: ExtensionConfiguration::new(values),
        })
    }

    pub(crate) fn persist(
        &self,
        extension_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = registered
            .get(extension_id)
            .expect("reserved configuration");
        let contribution = current.contribution.clone();
        let mut effective = current.values.clone();
        effective.insert(key.to_owned(), value);
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
        current.revision = self._revision.fetch_add(1, Ordering::Release) + 1;
        Ok(())
    }

    pub(crate) fn set_effective(
        &self,
        extension_id: &str,
        configuration: Option<ExtensionConfiguration>,
    ) {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = registered
            .get_mut(extension_id)
            .expect("reserved configuration");
        current.effective = configuration.map(ExtensionConfiguration::into_values);
        current.revision = self._revision.fetch_add(1, Ordering::Release) + 1;
    }

    pub(crate) fn outcome(
        &self,
        extension_id: &str,
        error: Option<String>,
    ) -> ConfigurationSaveOutcome {
        let registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = registered
            .get(extension_id)
            .expect("registered configuration");
        let visible = current
            .contribution
            .for_platform(nanika_platform::target_platform());
        let filter = |values: &BTreeMap<String, Value>| {
            values
                .iter()
                .filter(|(key, _)| visible.properties.contains_key(*key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        };
        ConfigurationSaveOutcome {
            revision: current.revision,
            values: filter(&Self::_values(current)),
            saved: filter(&current.values),
            effective: current.effective.as_ref().map(filter),
            error,
        }
    }

    pub(crate) fn close(&self) {
        self.operations.close();
    }

    pub(crate) fn wait_idle(&self) {
        self.operations.wait_idle();
    }

    /// Fresh processes receive durable intent, never a previous process's effective state.
    pub(crate) fn activation_configuration(
        &self,
        extension_id: &str,
    ) -> nanika_protocol::ExtensionConfiguration {
        let registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        registered
            .get(extension_id)
            .map(|entry| ExtensionConfiguration::new(entry.values.clone()))
            .unwrap_or_default()
    }

    pub(crate) fn retire(&self, extension_id: &str) {
        if let Some(current) = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(extension_id)
        {
            current.effective = None;
            current.revision = self._revision.fetch_add(1, Ordering::Release) + 1;
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self._revision.load(Ordering::Acquire)
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
                let filter = |values: &BTreeMap<String, Value>| {
                    values
                        .iter()
                        .filter(|(key, _)| contribution.properties.contains_key(*key))
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect()
                };
                let values = filter(&Self::_values(registered));
                let saved = filter(&registered.values);
                let effective = registered.effective.as_ref().map(filter);
                RuntimeExtensionConfiguration {
                    revision: registered.revision,
                    extension_id: extension_id.clone(),
                    contribution,
                    values,
                    saved,
                    effective,
                }
            })
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.extension_id.cmp(&right.extension_id));
        snapshots
    }
    fn _values(current: &RegisteredConfiguration) -> BTreeMap<String, Value> {
        let mut values = current.values.clone();
        if let Some(effective) = &current.effective {
            for (key, property) in &current.contribution.properties {
                if property.persistence == ConfigurationPersistence::AfterApply
                    && let Some(value) = effective.get(key)
                {
                    values.insert(key.clone(), value.clone());
                }
            }
        }
        values
    }
}
