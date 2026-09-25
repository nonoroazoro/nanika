use crate::{ConfigurationApplication, ConfigurationSaveOutcome, ExtensionConfigurationRegistry};
use nanika_extension_package::ConfigurationPersistence;
use nanika_protocol::ExtensionConfiguration;
use serde_json::Value;
use std::sync::Arc;

/// Owns the reservation across persistence, application, and final reconciliation.
pub(crate) struct ConfigurationOperation {
    pub(crate) _reservation: crate::ExtensionOperationReservation,
    pub(crate) registry: Arc<ExtensionConfigurationRegistry>,
    pub(crate) extension_id: String,
    pub(crate) key: String,
    pub(crate) value: Value,
    pub(crate) persistence: ConfigurationPersistence,
    pub(crate) configuration: ExtensionConfiguration,
}

impl ConfigurationOperation {
    pub(crate) fn run(
        self,
        apply: impl FnOnce(ExtensionConfiguration, bool) -> Result<ConfigurationApplication, String>,
    ) -> ConfigurationSaveOutcome {
        let error = self._run(apply).err();
        self.registry.outcome(&self.extension_id, error)
    }

    fn _run(
        &self,
        apply: impl FnOnce(ExtensionConfiguration, bool) -> Result<ConfigurationApplication, String>,
    ) -> Result<(), String> {
        let apply_first = self.persistence == ConfigurationPersistence::AfterApply;
        if !apply_first {
            self.registry
                .persist(&self.extension_id, &self.key, self.value.clone())?;
        }
        let applied = apply(self.configuration.clone(), apply_first);
        match applied {
            Ok(ConfigurationApplication::Applied) => {
                self.registry
                    .set_effective(&self.extension_id, Some(self.configuration.clone()));
                if apply_first {
                    // Persistence failure cannot roll back the confirmed effective state.
                    self.registry
                        .persist(&self.extension_id, &self.key, self.value.clone())
                        .map_err(|error| {
                            format!("The change took effect but could not be saved: {error}")
                        })?;
                }
                Ok(())
            }
            Ok(ConfigurationApplication::Deferred) if !apply_first => Ok(()),
            Ok(ConfigurationApplication::Deferred) => {
                Err("The extension is unavailable; the change was not saved.".to_owned())
            }
            Err(error) => {
                // A rejected request or disconnected process cannot prove its actual state.
                self.registry.set_effective(&self.extension_id, None);
                if apply_first {
                    Err(format!(
                        "The change could not be confirmed and was not saved: {error}"
                    ))
                } else {
                    Err(format!(
                        "The change was saved but could not be applied: {error}"
                    ))
                }
            }
        }
    }
}
