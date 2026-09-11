use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{ExtensionProtocol, resolve_active_extensions};
use nanika_search::{SearchHandle, SearchOwner, SearchSnapshot, UsageKey, UsageMap, UsageStat};
use nanika_storage::{ExtensionKind, NanikaPaths, SearchStorageWorker};

use crate::{
    DistributionInventory, ExtensionInvocationOutcome, ExtensionRuntime,
    ExtensionSearchCoordinator, HostServiceHandler, HostServiceRouter, RuntimeOutputUpdate,
    RuntimeSettingsUpdate, RuntimeUpdateBatch, RuntimeViewCompletion,
};

/// UI-independent owner for storage, search, extension processes, and host services.
pub struct RuntimeService {
    search_owner: Option<SearchOwner>,
    search: SearchHandle,
    extensions: ExtensionSearchCoordinator,
    storage: Option<SearchStorageWorker>,
    startup_diagnostics: Vec<String>,
}

impl RuntimeService {
    pub fn start(paths: &NanikaPaths, inventory_source: &str) -> Result<Self, String> {
        let inventory = DistributionInventory::parse(inventory_source)?;
        let mut diagnostics = Vec::new();
        let config_store = ConfigStore::open(paths.app_data_root(), paths.config_root())
            .map_err(|error| format!("configuration is unavailable: {error}"))?;
        let registry = ExtensionRegistryConfig::load(&config_store)
            .map_err(|error| format!("extension settings are unavailable: {error}"))?;
        let (storage_worker, storage_state) = SearchStorageWorker::spawn(paths.host_database())
            .map_err(|error| format!("host storage is unavailable: {error}"))?;
        let storage = Some(storage_worker);
        diagnostics.extend(storage_state.extension_errors);
        let usage = storage_state
            .usage
            .into_iter()
            .map(|stored| {
                (
                    UsageKey::new(
                        &stored.extension_id,
                        &stored.entry_id,
                        &stored.action_id,
                        &stored.query_context,
                    ),
                    UsageStat {
                        execution_count: stored.execution_count,
                        last_executed_at: stored.last_executed_at,
                    },
                )
            })
            .collect::<UsageMap>();
        let owner = SearchOwner::spawn(usage).map_err(|error| error.to_string())?;
        let search = owner.handle();
        if let Some(storage) = &storage {
            storage.attach_search(search.clone());
        }

        let (router, service_errors) = HostServiceRouter::spawn(paths.app_data_root());
        diagnostics.extend(service_errors);
        let router = Arc::new(router);
        let mut extensions = ExtensionSearchCoordinator::new();
        extensions.set_host_services(Arc::clone(&router) as Arc<dyn HostServiceHandler>);

        let current_executable = std::env::current_exe().map_err(|error| error.to_string())?;
        for extension in inventory.extensions {
            if !registry.is_enabled(&extension.id, true) {
                continue;
            }
            let program = companion_executable(&current_executable, &extension.binary_name);
            if !program.is_file() {
                diagnostics.push(format!(
                    "built-in extension executable is missing: {}",
                    program.display()
                ));
                continue;
            }
            router.register_permissions(&extension.id, extension.permissions);
            let runtime = match spawn_runtime(&extension.id, extension.runtime, &program, paths) {
                Ok(runtime) => runtime,
                Err(error) => {
                    diagnostics.push(format!(
                        "extension {} could not start: {error}",
                        extension.id
                    ));
                    continue;
                }
            };
            if let Some(storage) = &storage {
                storage
                    .register_extension(&extension.id, ExtensionKind::BuiltIn, unix_timestamp())
                    .map_err(|error| {
                        format!(
                            "extension {} metadata could not be recorded: {error}",
                            extension.id
                        )
                    })?;
            }
            if let Err(error) = extensions.register(
                &extension.id,
                runtime,
                search.clone(),
                extension.contributions,
            ) {
                diagnostics.push(format!(
                    "extension {} could not register: {error}",
                    extension.id
                ));
            }
        }

        let (external, errors) =
            resolve_active_extensions(paths, &storage_state.extensions, &registry);
        diagnostics.extend(errors.into_iter().map(|error| error.message));
        for extension in external {
            router.register_permissions(&extension.extension_id, extension.permissions);
            let runtime = match spawn_runtime(
                &extension.extension_id,
                extension.protocol,
                &extension.program,
                paths,
            ) {
                Ok(runtime) => runtime,
                Err(error) => {
                    diagnostics.push(format!(
                        "extension {} could not start: {error}",
                        extension.extension_id
                    ));
                    continue;
                }
            };
            if let Err(error) = extensions.register(
                &extension.extension_id,
                runtime,
                search.clone(),
                extension.contributions,
            ) {
                diagnostics.push(format!(
                    "extension {} could not register: {error}",
                    extension.extension_id
                ));
            }
        }
        Ok(Self {
            search_owner: Some(owner),
            search,
            extensions,
            storage,
            startup_diagnostics: diagnostics,
        })
    }

    pub fn begin_query(&self, query: impl Into<String>) -> Result<u64, String> {
        let query = query.into();
        let extension_ids = self.extensions.ready_extension_ids();
        let generation = self
            .search
            .begin_query_with_expected_extensions(query.clone(), extension_ids.iter().cloned())
            .map_err(|error| error.to_string())?;
        tracing::debug!(
            generation,
            expected_extensions = extension_ids.len(),
            extension_ids = ?extension_ids,
            "search query dispatched"
        );
        // Pending workers retain only the latest query and publish it once ready.
        // They must not hold the barrier for extensions that can already answer.
        self.extensions.query(generation, &query);
        Ok(generation)
    }

    pub fn latest_snapshot(&self) -> Option<Arc<SearchSnapshot>> {
        self.search.latest_snapshot()
    }

    pub fn set_notifier(&self, notifier: Arc<dyn Fn() + Send + Sync>) {
        self.search.set_notifier(Arc::clone(&notifier));
        self.extensions.set_notifier(notifier);
    }

    pub fn invoke(
        &self,
        generation: u64,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
    ) -> Result<std::sync::mpsc::Receiver<Result<ExtensionInvocationOutcome, String>>, String> {
        let snapshot = self
            .latest_snapshot()
            .filter(|snapshot| snapshot.generation == generation)
            .ok_or_else(|| "the selected search generation is no longer active".to_owned())?;
        let candidate = snapshot
            .results
            .iter()
            .map(|result| &result.candidate)
            .find(|candidate| {
                candidate.extension_id() == extension_id
                    && candidate.entry_id() == entry_id
                    && candidate.action_id() == action_id
            })
            .ok_or_else(|| "the selected candidate is no longer available".to_owned())?;
        self.extensions
            .invoke(
                extension_id,
                generation,
                candidate.entry_id(),
                candidate.action_id(),
                query_context,
            )
            .map_err(|error| error.to_string())
    }

    pub fn record_execution(
        &self,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
    ) -> Result<(), String> {
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| "host storage is unavailable".to_owned())?;
        let result = if query_context.trim().is_empty() {
            storage.record_usage(
                extension_id,
                entry_id,
                action_id,
                query_context,
                unix_timestamp(),
            )
        } else {
            storage.record_execution(
                nanika_search::normalize_history_key(query_context),
                query_context,
                UsageKey::new(extension_id, entry_id, action_id, query_context),
                unix_timestamp_millis(),
                unix_timestamp(),
            )
        };
        result.map_err(|error| format!("could not record completed action: {error}"))
    }

    pub fn startup_diagnostics(&self) -> &[String] {
        &self.startup_diagnostics
    }

    pub fn search_warnings(&self) -> Vec<String> {
        let mut warnings = self.startup_diagnostics.clone();
        warnings.extend(self.extensions.warnings());
        warnings
    }

    pub fn active_error(&self) -> Option<String> {
        self.storage
            .as_ref()
            .and_then(SearchStorageWorker::last_failure)
            .map(|failure| {
                format!(
                    "Host storage failed while trying to {}. Open diagnostics for details.",
                    failure.operation()
                )
            })
    }

    pub fn view_event(
        &self,
        extension_id: &str,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent,
    ) -> Result<std::sync::mpsc::Receiver<Result<RuntimeViewCompletion, String>>, String> {
        self.extensions
            .view_event(extension_id, generation, view_id, revision, event)
            .map_err(|error| error.to_string())
    }

    pub fn close_view(
        &self,
        extension_id: &str,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
    ) -> Result<std::sync::mpsc::Receiver<Result<RuntimeViewCompletion, String>>, String> {
        self.extensions
            .close_view(extension_id, generation, view_id, revision)
            .map_err(|error| error.to_string())
    }

    pub fn update_settings(
        &self,
        extension_id: &str,
        request_id: impl Into<String>,
        updates: Vec<nanika_protocol::SettingUpdate>,
    ) -> Result<(), String> {
        self.extensions
            .update_settings(extension_id, request_id, updates)
            .map_err(|error| error.to_string())
    }

    pub fn take_updates(&self) -> RuntimeUpdateBatch {
        let outputs = self
            .extensions
            .take_invocation_outputs()
            .into_iter()
            .map(|update| RuntimeOutputUpdate {
                invocation_id: update.invocation_id,
                extension_id: update.extension_id,
                generation: update.generation,
                text: update.text,
            })
            .collect();
        let settings = self
            .extensions
            .take_settings()
            .into_iter()
            .map(|update| RuntimeSettingsUpdate {
                extension_id: update.extension_id,
                request_id: update.request_id,
                result: update.result,
            })
            .collect();
        RuntimeUpdateBatch { outputs, settings }
    }
}

impl Drop for RuntimeService {
    fn drop(&mut self) {
        self.extensions.shutdown();
        if let Some(storage) = self.storage.take() {
            storage.shutdown();
        }
        if let Some(owner) = self.search_owner.take() {
            owner.shutdown();
        }
    }
}

fn spawn_runtime(
    extension_id: &str,
    protocol: ExtensionProtocol,
    program: &Path,
    paths: &NanikaPaths,
) -> Result<ExtensionRuntime, std::io::Error> {
    ExtensionRuntime::spawn_with(
        extension_id,
        protocol,
        program,
        extension_arguments(protocol, paths),
        Default::default(),
    )
}

fn companion_executable(current_executable: &Path, binary_name: &str) -> PathBuf {
    current_executable.with_file_name(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX))
}

fn extension_arguments(protocol: ExtensionProtocol, paths: &NanikaPaths) -> Vec<OsString> {
    match protocol {
        ExtensionProtocol::Nanika {
            protocol_version: 1,
        } => vec![
            path_argument("data-root", paths.app_data_root()),
            path_argument("cache-root", paths.cache_root()),
            path_argument("config-root", paths.config_root()),
        ],
        _ => Vec::new(),
    }
}

fn path_argument(name: &str, path: &Path) -> OsString {
    OsString::from(format!("--{name}={}", path.display()))
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn unix_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}
