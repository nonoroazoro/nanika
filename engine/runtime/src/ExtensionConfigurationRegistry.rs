use std::collections::{BTreeMap, HashMap};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

use nanika_config::{ConfigStore, ExtensionConfigurationFile};
use nanika_extension_package::{ConfigurationContribution, ConfigurationPersistence};
use nanika_protocol::ExtensionConfiguration;
use serde_json::Value;

use crate::{ConfigurationOperation, ConfigurationSaveOutcome, RuntimeExtensionConfiguration};

#[derive(Debug, Clone)]
struct RegisteredConfiguration {
    contribution: ConfigurationContribution,
    values: BTreeMap<String, Value>,
    effective: Option<BTreeMap<String, Value>>,
    busy: bool,
}

pub(crate) struct ExtensionConfigurationRegistry {
    store: ConfigStore,
    registered: Mutex<HashMap<String, RegisteredConfiguration>>,
    idle: Condvar,
    closed: AtomicBool,
}

impl ExtensionConfigurationRegistry {
    pub(crate) fn new(store: ConfigStore) -> Self {
        Self {
            store,
            registered: Mutex::new(HashMap::new()),
            idle: Condvar::new(),
            closed: AtomicBool::new(false),
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
                    effective: None,
                    busy: false,
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
        if self.closed.load(Ordering::Acquire) {
            return Err("Configuration service is shutting down.".to_owned());
        }
        let current = registered.get_mut(extension_id).ok_or_else(|| {
            format!("extension does not contribute configuration: {extension_id}")
        })?;
        if current.busy {
            return Err(
                "A configuration operation is already pending for this extension.".to_owned(),
            );
        }
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
        current.busy = true;
        Ok(ConfigurationOperation {
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
        registered
            .get_mut(extension_id)
            .expect("reserved configuration")
            .effective = configuration.map(ExtensionConfiguration::into_values);
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
            values: filter(&Self::_values(current)),
            saved: filter(&current.values),
            effective: current.effective.as_ref().map(filter),
            error,
        }
    }

    pub(crate) fn release(&self, extension_id: &str) {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        registered
            .get_mut(extension_id)
            .expect("reserved configuration")
            .busy = false;
        self.idle.notify_all();
    }

    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    pub(crate) fn wait_idle(&self) {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while registered.values().any(|entry| entry.busy) {
            registered = self
                .idle
                .wait(registered)
                .unwrap_or_else(|error| error.into_inner());
        }
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
