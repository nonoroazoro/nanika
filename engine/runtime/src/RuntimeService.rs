use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{CommandMode, ExtensionProtocol, resolve_active_extensions};
use nanika_search::{SearchHandle, SearchOwner, SearchSnapshot, UsageKey, UsageMap, UsageStat};
use nanika_storage::{ExtensionKind, NanikaPaths, SearchStorageWorker};

use crate::{
    DistributionInventory, ExtensionInvocationOutcome, ExtensionRuntime,
    ExtensionSearchCoordinator, HostServiceHandler, HostServiceRouter, RuntimeInvocationCompletion,
    RuntimeInvocationUpdate, RuntimeOutputUpdate, RuntimeSettingsUpdate, RuntimeUpdateBatch,
    RuntimeViewCompletion, RuntimeViewUpdate,
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
        let registry = match ConfigStore::open(paths.app_data_root(), paths.config_root()) {
            Ok(store) => match ExtensionRegistryConfig::load(&store) {
                Ok(registry) => registry,
                Err(error) => {
                    diagnostics.push(format!("extension settings are unavailable: {error}"));
                    ExtensionRegistryConfig::default()
                }
            },
            Err(error) => {
                diagnostics.push(format!("configuration is unavailable: {error}"));
                ExtensionRegistryConfig::default()
            }
        };
        let (storage, storage_state) = match SearchStorageWorker::spawn(paths.host_database(), 50) {
            Ok((worker, state)) => (Some(worker), state),
            Err(error) => {
                diagnostics.push(format!("host storage is unavailable: {error}"));
                (None, Default::default())
            }
        };
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
                let _ = storage.register_extension(
                    &extension.id,
                    ExtensionKind::BuiltIn,
                    unix_timestamp(),
                );
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
        let generation = self
            .search
            .begin_query_with_expected_extensions(
                query.clone(),
                self.extensions.ready_query_count(),
            )
            .map_err(|error| error.to_string())?;
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
    ) -> Result<bool, String> {
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
            .map_err(|error| error.to_string())?;
        if !query_context.trim().is_empty()
            && let Some(storage) = &self.storage
        {
            let _ = storage.record_history(
                nanika_search::normalize_history_key(query_context),
                query_context,
                unix_timestamp_millis(),
            );
            let _ = storage.record_usage(
                extension_id,
                entry_id,
                action_id,
                query_context,
                unix_timestamp(),
            );
        }
        Ok(matches!(
            self.extensions.command_mode(extension_id, entry_id),
            Some(CommandMode::View)
        ))
    }

    pub fn startup_diagnostics(&self) -> &[String] {
        &self.startup_diagnostics
    }

    pub fn view_event(
        &self,
        extension_id: &str,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent,
    ) -> Result<u64, String> {
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
    ) -> Result<u64, String> {
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
        let invocations = self
            .extensions
            .take_results()
            .into_iter()
            .map(|update| RuntimeInvocationUpdate {
                invocation_id: update.invocation_id,
                extension_id: update.extension_id,
                generation: update.generation,
                entry_id: update.entry_id,
                action_id: update.action_id,
                query_context: update.query_context,
                result: update.result.map(|outcome| match outcome {
                    ExtensionInvocationOutcome::Completed { effect, has_output } => {
                        RuntimeInvocationCompletion {
                            effect: Some(effect),
                            has_output,
                            cancelled: false,
                        }
                    }
                    ExtensionInvocationOutcome::Cancelled => RuntimeInvocationCompletion {
                        effect: None,
                        has_output: false,
                        cancelled: true,
                    },
                }),
            })
            .collect();
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
        let views = self
            .extensions
            .take_view_updates()
            .into_iter()
            .map(|update| RuntimeViewUpdate {
                request_id: update.request_id,
                extension_id: update.extension_id,
                generation: update.generation,
                view_id: update.view_id,
                result: update.result.map(|completion| RuntimeViewCompletion {
                    revision: completion.revision,
                    effect: completion.effect,
                    view: completion.view,
                }),
            })
            .collect();
        RuntimeUpdateBatch {
            invocations,
            outputs,
            settings,
            views,
        }
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
