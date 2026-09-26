use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, Mutex};

use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{
    ExtensionProtocol, InstalledExtension, resolve_installed_extensions,
};
use nanika_platform::companion_executable;
use nanika_search::{SearchHandle, SearchOwner, SearchSnapshot, UsageKey, UsageMap, UsageStat};
use nanika_storage::{NanikaPaths, SearchStorageWorker};

use crate::{
    BuiltInExtensionInventory, ExtensionConfigurationRegistry, ExtensionInvocationOutcome,
    ExtensionRuntime, ExtensionSearchCoordinator, HostServiceHandler, HostServiceRouter,
    RuntimeOutputUpdate, RuntimeUpdateBatch, RuntimeViewCompletion,
};

/// UI-independent owner for storage, search, extension processes, and host services.
pub struct RuntimeService {
    search_owner: Mutex<Option<SearchOwner>>,
    search: SearchHandle,
    extensions: Arc<ExtensionSearchCoordinator>,
    extension_info: Mutex<Vec<crate::RuntimeExtensionInfo>>,
    installed: std::collections::HashMap<String, InstalledExtension>,
    config_store: ConfigStore,
    paths: NanikaPaths,
    router: Arc<HostServiceRouter>,
    current_query: Mutex<Option<(u64, String)>>,
    configurations: Arc<ExtensionConfigurationRegistry>,
    storage: Option<SearchStorageWorker>,
    startup_diagnostics: Vec<String>,
    _notifier: crate::ExtensionNotifier,
    _recovery_wake: std::sync::mpsc::SyncSender<()>,
    _supervisor: Mutex<Option<std::thread::JoinHandle<()>>>,
    _closing: std::sync::atomic::AtomicBool,
    _restarted: Mutex<std::collections::HashMap<String, String>>,
}

impl RuntimeService {
    pub fn start(
        paths: &NanikaPaths,
        built_in_manifests: &[&str],
        built_in_resources: &Path,
    ) -> Result<Arc<Self>, String> {
        let inventory = BuiltInExtensionInventory::parse(built_in_manifests)?;
        let mut diagnostics = Vec::new();
        let config_store = ConfigStore::open(paths.app_data_root(), paths.config_root())
            .map_err(|error| format!("configuration is unavailable: {error}"))?;
        let registry = ExtensionRegistryConfig::load(&config_store)
            .map_err(|error| format!("extension registry is unavailable: {error}"))?;
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
        let extensions = ExtensionSearchCoordinator::new();
        extensions.set_host_services(Arc::clone(&router) as Arc<dyn HostServiceHandler>);
        let configurations = Arc::new(ExtensionConfigurationRegistry::new(config_store.clone()));

        let current_executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let mut installed_extensions = Vec::new();
        for extension in inventory.extensions {
            let manifest = extension.manifest;
            let program = companion_executable(&current_executable, &extension.binary_name);
            if !program.is_file() {
                diagnostics.push(format!(
                    "built-in extension executable is missing: {}",
                    program.display()
                ));
                continue;
            }
            if let Some(storage) = &storage {
                storage
                    .register_builtin_extension(&manifest.id)
                    .map_err(|error| {
                        format!(
                            "extension {} metadata could not be recorded: {error}",
                            manifest.id
                        )
                    })?;
            }
            let resource_root = built_in_resources.join(&manifest.id);
            installed_extensions.push(InstalledExtension::from_manifest(
                manifest,
                program,
                resource_root,
            ));
        }

        let (mut external, errors) = resolve_installed_extensions(paths, &storage_state.extensions);
        diagnostics.extend(errors.into_iter().map(|error| error.message));
        installed_extensions.append(&mut external);
        let installed = installed_extensions
            .iter()
            .map(|extension| (extension.extension_id.clone(), extension.clone()))
            .collect();
        let mut extension_info = Vec::new();
        for extension in installed_extensions {
            let enabled = registry.is_enabled(&extension.extension_id);
            let configuration = configurations.register(
                &extension.extension_id,
                extension.contributes.configuration.as_ref(),
            );
            // Installed metadata remains available without a live worker or schema.
            extension_info.push(crate::RuntimeExtensionInfo {
                id: extension.extension_id.clone(),
                name: extension.name,
                icon: extension.icon,
                enabled,
                pending: false,
                state: if enabled {
                    crate::RuntimeExtensionState::Starting
                } else {
                    crate::RuntimeExtensionState::Disabled
                },
                instance_id: None,
                lifecycle_error: None,
                configuration_error: configuration.as_ref().err().cloned(),
            });
            if let Err(error) = configuration {
                diagnostics.push(format!(
                    "extension {} configuration is unavailable: {error}",
                    extension.extension_id
                ));
            }
        }
        extension_info.sort_by(|left, right| left.id.cmp(&right.id));
        let (recovery_wake, recovery) = std::sync::mpsc::sync_channel(1);
        let released = recovery_wake.clone();
        configurations
            .operations
            .set_release_notifier(Arc::new(move || {
                let _ = released.try_send(());
            }));
        let notifier: crate::ExtensionNotifier = Arc::new(Mutex::new(None));
        let notify = Arc::clone(&notifier);
        let wake = recovery_wake.clone();
        extensions.set_notifier(Arc::new(move || {
            // Coalesced wakes request an authoritative scan, not an operation replay.
            let _ = wake.try_send(());
            let notify = notify
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if let Some(notify) = notify {
                notify();
            }
        }));
        let mut runtime = Self {
            search_owner: Mutex::new(Some(owner)),
            search,
            extensions: Arc::new(extensions),
            extension_info: Mutex::new(extension_info),
            installed,
            config_store,
            paths: paths.clone(),
            router,
            current_query: Mutex::new(None),
            configurations,
            storage,
            startup_diagnostics: diagnostics,
            _notifier: notifier,
            _recovery_wake: recovery_wake,
            _supervisor: Mutex::new(None),
            _closing: std::sync::atomic::AtomicBool::new(false),
            _restarted: Mutex::new(std::collections::HashMap::new()),
        };
        for info in runtime.extension_info() {
            if info.enabled
                && info.configuration_error.is_none()
                && let Err(error) = runtime._activate_extension(&info.id, false)
            {
                runtime
                    .extension_info
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .iter_mut()
                    .find(|item| item.id == info.id)
                    .expect("installed metadata")
                    .lifecycle_error = Some(error.clone());
                runtime
                    .startup_diagnostics
                    .push(format!("extension {} could not start: {error}", info.id));
            }
        }
        let runtime = Arc::new(runtime);
        let weak = Arc::downgrade(&runtime);
        let supervisor = std::thread::Builder::new()
            .name("extension-supervisor".into())
            .spawn(move || {
                while recovery.recv().is_ok() {
                    let Some(runtime) = weak.upgrade() else {
                        break;
                    };
                    if runtime._closing.load(std::sync::atomic::Ordering::Acquire) {
                        break;
                    }
                    runtime._recover_extensions();
                }
            })
            .map_err(|error| error.to_string())?;
        *runtime
            ._supervisor
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(supervisor);
        let _ = runtime._recovery_wake.try_send(());
        Ok(runtime)
    }

    pub fn begin_query(&self, query: impl Into<String>) -> Result<u64, String> {
        let query = query.into();
        // Query admission and withdrawal share this short barrier. A query must
        // never retain an expected contributor after that contributor is retired.
        let mut current_query = self
            .current_query
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
        // Pending workers publish the latest query when ready without blocking ready extensions.
        *current_query = Some((generation, query.clone()));
        self.extensions.query(generation, &query);
        Ok(generation)
    }

    /// Wait for catalog refresh completion from dynamic Root Search extensions.
    pub fn refresh_root_search(&self, generation: u64) -> Result<(), String> {
        self.extensions.refresh_root_search(generation)
    }

    pub fn latest_snapshot(&self) -> Option<Arc<SearchSnapshot>> {
        self.search.latest_snapshot()
    }

    pub fn prepare_visible_entries(&self, snapshot: &SearchSnapshot, limit: usize) {
        let count = limit.min(snapshot.results.len());
        self.extensions
            .prepare_entries(snapshot.generation, &snapshot.results[..count]);
    }

    pub fn take_view_invalidations(&self) -> Vec<crate::RuntimeViewInvalidation> {
        self.extensions.take_view_invalidations()
    }

    pub fn set_notifier(&self, notifier: Arc<dyn Fn() + Send + Sync>) {
        self.search.set_notifier(Arc::clone(&notifier));
        *self
            ._notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notifier);
    }

    pub fn invoke(
        &self,
        snapshot: &Arc<SearchSnapshot>,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
        invocation: nanika_protocol::ActionInvocation,
    ) -> Result<std::sync::mpsc::Receiver<Result<ExtensionInvocationOutcome, String>>, String> {
        if !self
            .latest_snapshot()
            .is_some_and(|current| Arc::ptr_eq(&current, snapshot))
        {
            return Err("Search changed. Select a current result.".to_owned());
        }
        let candidate = snapshot
            .results
            .iter()
            .map(|result| &result.candidate)
            .find(|candidate| {
                candidate.extension_id() == extension_id
                    && candidate.entry_id() == entry_id
                    && (invocation != nanika_protocol::ActionInvocation::Default
                        || candidate.action_id() == action_id)
                    && candidate.actions().iter().any(|action| {
                        action.id == action_id && action.allows_invocation(invocation)
                    })
            })
            .ok_or_else(|| "the selected candidate is no longer available".to_owned())?;
        let instance_id = self
            .instance_id(extension_id)
            .ok_or("The extension is disabled.")?;
        if !self
            .latest_snapshot()
            .is_some_and(|current| Arc::ptr_eq(&current, snapshot))
        {
            return Err("Search changed. Select a current result.".into());
        }
        self.extensions
            .invoke(
                extension_id,
                instance_id,
                snapshot.generation,
                candidate.entry_id(),
                action_id,
                query_context,
            )
            .map_err(|error| error.to_string())
    }

    /// Complete an accepted action and report recording independently of its outcome.
    /// A recording failure must not discard an already created extension view.
    pub fn invoke_recorded(
        &self,
        snapshot: &Arc<SearchSnapshot>,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
        invocation: nanika_protocol::ActionInvocation,
    ) -> Result<crate::RuntimeInvocationCompletion, String> {
        let instance_id = self
            .instance_id(extension_id)
            .ok_or("The extension is disabled.")?;
        let is_default = snapshot.results.iter().any(|result| {
            let candidate = &result.candidate;
            candidate.extension_id() == extension_id
                && candidate.entry_id() == entry_id
                && candidate.action_id() == action_id
        });
        let outcome = self
            .invoke(
                snapshot,
                extension_id,
                entry_id,
                action_id,
                query_context,
                invocation,
            )?
            .recv()
            .map_err(|_| "Extension closed without an invocation result.".to_owned())??;
        let recording_error =
            if is_default && matches!(outcome, ExtensionInvocationOutcome::Completed { .. }) {
                self.record_execution(extension_id, entry_id, action_id, query_context)
                    .err()
            } else {
                None
            };
        Ok(crate::RuntimeInvocationCompletion {
            instance_id,
            outcome,
            recording_error,
        })
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

    pub fn request_shutdown(&self) {
        self._closing
            .store(true, std::sync::atomic::Ordering::Release);
        self.configurations.close();
        let _ = self._recovery_wake.try_send(());
        self.extensions.request_shutdown();
    }

    /// Call after application operations settle; ownership is shared with workers.
    pub fn shutdown(&self) {
        self.request_shutdown();
        let mut owner = self
            .search_owner
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.configurations.close();
        self.extensions.shutdown();
        self.configurations.wait_idle();
        if let Some(storage) = &self.storage {
            storage.shutdown();
        }
        if let Some(owner) = owner.take() {
            owner.shutdown();
        }
        drop(owner);
        if let Some(supervisor) = self
            ._supervisor
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && supervisor.thread().id() != std::thread::current().id()
        {
            let _ = supervisor.join();
        }
    }

    pub fn configuration_revision(&self) -> u64 {
        self.configurations.revision()
    }

    pub fn extension_configurations(&self) -> Vec<crate::RuntimeExtensionConfiguration> {
        self.configurations.snapshots()
    }

    /// Installed presentation metadata remains available without a live process.
    pub fn extension_icon(&self, extension_id: &str) -> Option<nanika_protocol::IconSource> {
        self.installed
            .get(extension_id)
            .map(|extension| nanika_protocol::IconSource::Package {
                path: extension.icon.clone(),
            })
    }

    pub fn extension_resource_roots(
        &self,
    ) -> std::collections::HashMap<String, std::path::PathBuf> {
        self.installed
            .iter()
            .map(|(id, extension)| (id.clone(), extension.resource_root.clone()))
            .collect()
    }

    pub fn extension_info(&self) -> Vec<crate::RuntimeExtensionInfo> {
        use crate::RuntimeExtensionState as State;
        let mut infos = self
            .extension_info
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        for info in &mut infos {
            if let Some(worker) = self.extensions.worker(&info.id) {
                info.instance_id = Some(worker.instance.id);
                if info.enabled && !matches!(info.state, State::Stopping | State::Failed) {
                    let (state, error) = worker.lifecycle();
                    info.state = state;
                    info.lifecycle_error = error.or_else(|| info.lifecycle_error.take());
                }
            } else if info.enabled && !info.pending {
                info.state = State::Failed;
                if info.lifecycle_error.is_none() {
                    info.lifecycle_error = info
                        .configuration_error
                        .clone()
                        .or_else(|| Some("The extension could not start.".into()));
                }
            }
        }
        infos
    }

    pub fn instance_id(&self, extension_id: &str) -> Option<u64> {
        self.extensions
            .worker(extension_id)
            .filter(|worker| worker.instance.is_active())
            .map(|worker| worker.instance.id)
    }

    /// Execute a short publication while its originating instance remains admitted.
    /// The callback must not wait for extension work or call back into this instance.
    pub fn with_extension_instance<T>(
        &self,
        extension_id: &str,
        instance_id: u64,
        publish: impl FnOnce() -> T,
    ) -> Option<T> {
        let worker = self.extensions.worker(extension_id)?;
        if worker.instance.id != instance_id {
            return None;
        }
        worker.instance.with_active(publish)
    }

    /// Admission is synchronous; persistence and process waits run off the caller thread.
    pub fn set_extension_enabled(
        self: &Arc<Self>,
        extension_id: &str,
        enabled: bool,
    ) -> Result<std::sync::mpsc::Receiver<Result<(), String>>, String> {
        if !self.installed.contains_key(extension_id) {
            return Err("Unknown installed extension.".into());
        }
        let reservation = self.configurations.operations.reserve(extension_id)?;
        let runtime = Arc::clone(self);
        let extension_id = extension_id.to_owned();
        let (complete, receipt) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("extension-lifecycle".into())
            .spawn(move || {
                let _reservation = reservation;
                {
                    let mut infos = runtime
                        .extension_info
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    infos
                        .iter_mut()
                        .find(|info| info.id == extension_id)
                        .expect("installed metadata")
                        .pending = true;
                }
                runtime.extensions.notify();
                let result = runtime._set_extension_enabled(&extension_id, enabled);
                {
                    let mut infos = runtime
                        .extension_info
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    infos
                        .iter_mut()
                        .find(|info| info.id == extension_id)
                        .expect("installed metadata")
                        .pending = false;
                }
                if let Err(error) = &result {
                    let mut infos = runtime
                        .extension_info
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if let Some(info) = infos.iter_mut().find(|info| info.id == extension_id) {
                        info.lifecycle_error = Some(error.clone());
                        if info.state == crate::RuntimeExtensionState::Stopping {
                            info.state = crate::RuntimeExtensionState::Failed;
                        }
                    }
                }
                drop(_reservation);
                runtime.extensions.notify();
                let _ = complete.send(result);
            })
            .map_err(|error| error.to_string())?;
        Ok(receipt)
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
        instance_id: u64,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent,
    ) -> Result<std::sync::mpsc::Receiver<Result<RuntimeViewCompletion, String>>, String> {
        self.extensions
            .view_event(
                extension_id,
                instance_id,
                generation,
                view_id,
                revision,
                event,
            )
            .map_err(|error| error.to_string())
    }

    pub fn close_view(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
    ) -> Result<std::sync::mpsc::Receiver<Result<RuntimeViewCompletion, String>>, String> {
        self.extensions
            .close_view(extension_id, instance_id, generation, view_id, revision)
            .map_err(|error| error.to_string())
    }

    /// Admit one property operation per extension. The runtime owns its completion,
    /// including persistence after application when required by the property contract.
    pub fn save_configuration(
        &self,
        extension_id: &str,
        request_id: impl Into<String>,
        key: String,
        value: serde_json::Value,
        progress: crate::ConfigurationProgressHandler,
    ) -> Result<crate::ConfigurationSaveReceipt, String> {
        let operation = self.configurations.prepare(extension_id, key, value)?;
        let extensions = Arc::clone(&self.extensions);
        let extension_id = extension_id.to_owned();
        let request_id = request_id.into();
        let (completion, received) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("configuration-operation".to_owned())
            .spawn(move || {
                let outcome = operation.run(|configuration, require_live| {
                    let (applied, application) = std::sync::mpsc::sync_channel(1);
                    match extensions.apply_configuration(
                        &extension_id,
                        request_id,
                        configuration,
                        require_live,
                        progress,
                        applied,
                    ) {
                        Ok(true) => application.recv().map_err(|_| {
                            "Extension closed without a configuration result.".to_owned()
                        })?,
                        Ok(false) => Ok(crate::ConfigurationApplication::Deferred),
                        Err(error) => Err(error.to_string()),
                    }
                });
                extensions.notify();
                let _ = completion.send(outcome);
            })
            .map_err(|error| error.to_string())?;
        Ok(crate::ConfigurationSaveReceipt(received))
    }

    pub fn take_updates(&self) -> RuntimeUpdateBatch {
        let outputs = self
            .extensions
            .take_invocation_outputs()
            .into_iter()
            .map(|update| RuntimeOutputUpdate {
                instance_id: update.instance_id,
                invocation_id: update.invocation_id,
                extension_id: update.extension_id,
                generation: update.generation,
                text: update.text,
            })
            .collect();
        RuntimeUpdateBatch { outputs }
    }
    fn _set_extension_enabled(&self, extension_id: &str, enabled: bool) -> Result<(), String> {
        use crate::RuntimeExtensionState as State;
        if enabled && let Some(worker) = self.extensions.worker(extension_id) {
            if worker.instance.is_active() {
                return Ok(());
            }
            return Err("The previous extension instance has not stopped.".into());
        }
        {
            let mut transaction =
                nanika_config::ExtensionRegistryTransaction::begin(&self.config_store)?;
            transaction.set_enabled(extension_id, enabled);
            transaction.save()?;
        }
        {
            let mut infos = self
                .extension_info
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let info = infos
                .iter_mut()
                .find(|info| info.id == extension_id)
                .expect("installed metadata");
            info.enabled = enabled;
            info.state = if enabled {
                State::Starting
            } else {
                State::Stopping
            };
            info.lifecycle_error = None;
        }
        self.extensions.notify();
        if !enabled {
            self.configurations.retire(extension_id);
            if let Some(worker) = self.extensions.worker(extension_id) {
                {
                    let _query = self
                        .current_query
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    worker.retire()?;
                }
                self.extensions.notify();
                worker.wait_stopped()?;
                self.extensions.remove_stopped(extension_id);
            }
            let mut infos = self
                .extension_info
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let info = infos
                .iter_mut()
                .find(|info| info.id == extension_id)
                .expect("installed metadata");
            info.state = State::Disabled;
            info.instance_id = None;
            return Ok(());
        }
        self._restarted
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(extension_id);
        self._activate_extension(extension_id, false)
    }

    fn _recover_extensions(&self) {
        use crate::RuntimeExtensionState as State;
        for info in self.extension_info() {
            if !info.enabled || info.pending || info.state != State::Failed {
                continue;
            }
            let Some(worker) = self.extensions.worker(&info.id) else {
                continue;
            };
            if Some(worker.instance.id) != info.instance_id
                || worker.lifecycle().0 != State::Failed
                || !worker.instance.is_active()
            {
                continue;
            }
            // Every reservation release wakes recovery, including rejected validation
            // and failed thread creation. There is no dependency on a success reply.
            let Ok(reservation) = self.configurations.operations.reserve(&info.id) else {
                continue;
            };
            if !worker.instance.is_active() {
                continue;
            }
            let cause = info
                .lifecycle_error
                .unwrap_or_else(|| "Extension worker failed.".into());
            let restarting = !self
                ._restarted
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .contains_key(&info.id);
            {
                let mut infos = self
                    .extension_info
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let current = infos
                    .iter_mut()
                    .find(|item| item.id == info.id)
                    .expect("installed metadata");
                current.pending = true;
                current.state = if restarting {
                    State::Starting
                } else {
                    State::Stopping
                };
                current.lifecycle_error = Some(if restarting {
                    format!("Extension failed: {cause}. Restarting once.")
                } else {
                    format!("Extension failed again: {cause}. Stopping the failed instance.")
                });
            }
            self.extensions.notify();
            self.configurations.retire(&info.id);
            // Revoke the old instance before terminating its remaining containment.
            // Accepted actions receive a terminal failure and are never replayed.
            worker.request_stop();
            let retired = {
                let _query = self
                    .current_query
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                worker.retire_after_failure(&cause)
            };
            let stopped = worker.wait_stopped();
            let cause = worker.lifecycle().1.unwrap_or(cause);
            let previous = self
                ._restarted
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(info.id.clone(), cause.clone());
            let result = retired.and(stopped).and_then(|()| {
                self.extensions.remove_stopped(&info.id);
                if let Some(first) = previous {
                    return Err(format!("Extension failed again after one automatic restart. First failure: {first}. Restarted instance: {cause}. Automatic restart stopped; disable and enable the extension to try again."));
                }
                if self._closing.load(std::sync::atomic::Ordering::Acquire) {
                    return Err("Application is shutting down.".into());
                }
                self._activate_extension(&info.id, true).map_err(|error|
                    format!("Automatic restart could not start. Original failure: {cause}. Restart failure: {error}."))
            });
            {
                let mut infos = self
                    .extension_info
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let current = infos
                    .iter_mut()
                    .find(|item| item.id == info.id)
                    .expect("installed metadata");
                current.pending = false;
                current.instance_id = None;
                match result {
                    Ok(()) => {
                        current.state = State::Starting;
                        current.lifecycle_error = None;
                    }
                    Err(error) => {
                        current.state = State::Failed;
                        current.lifecycle_error =
                            Some(format!("Extension recovery stopped: {error}"));
                    }
                }
            }
            drop(reservation);
            self.extensions.notify();
        }
    }

    /// Startup and explicit enable use the same worker-owned activation path.
    fn _activate_extension(&self, extension_id: &str, restarting: bool) -> Result<(), String> {
        let descriptor = &self.installed[extension_id];
        let needs_configuration = self
            .extension_info
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .find(|info| info.id == extension_id)
            .expect("installed metadata")
            .configuration_error
            .is_some();
        if needs_configuration {
            // Explicit activation can re-read a configuration the user has corrected.
            let result = self
                .configurations
                .register(extension_id, descriptor.contributes.configuration.as_ref());
            self.extension_info
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .iter_mut()
                .find(|info| info.id == extension_id)
                .expect("installed metadata")
                .configuration_error = result.as_ref().err().cloned();
            result?;
        }
        let configuration = self.configurations.activation_configuration(extension_id);
        self.router
            .register_permissions(extension_id, descriptor.permissions.clone());
        let owned_descriptor = descriptor.clone();
        let paths = self.paths.clone();
        let source = crate::ExtensionRuntimeSource::Factory {
            activation: if restarting {
                nanika_extension_package::ExtensionActivation::Startup
            } else {
                descriptor.activation
            },
            live_configuration: matches!(descriptor.protocol, ExtensionProtocol::Nanika { .. }),
            start: Box::new(move |configuration| {
                spawn_runtime(
                    &owned_descriptor.extension_id,
                    owned_descriptor.protocol,
                    &owned_descriptor.program,
                    &paths,
                    configuration,
                )
            }),
        };
        self.extensions
            .register_source(
                extension_id,
                source,
                self.search.clone(),
                descriptor.contributes.clone(),
                configuration,
            )
            .map_err(|error| error.to_string())?;
        if let Some((generation, query)) = self
            .current_query
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            && let Some(worker) = self.extensions.worker(extension_id)
        {
            worker.query(generation, query);
        }
        Ok(())
    }
}

impl Drop for RuntimeService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_runtime(
    extension_id: &str,
    protocol: ExtensionProtocol,
    program: &Path,
    paths: &NanikaPaths,
    configuration: nanika_protocol::ExtensionConfiguration,
) -> Result<ExtensionRuntime, std::io::Error> {
    ExtensionRuntime::spawn_with_configuration(
        extension_id,
        protocol,
        program,
        extension_arguments(protocol, paths),
        Default::default(),
        configuration,
    )
}

fn extension_arguments(protocol: ExtensionProtocol, paths: &NanikaPaths) -> Vec<OsString> {
    match protocol {
        ExtensionProtocol::Nanika {
            protocol_version: 1,
        } => vec![
            path_argument("data-root", paths.app_data_root()),
            path_argument("cache-root", paths.cache_root()),
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
