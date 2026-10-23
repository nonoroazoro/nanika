use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, Mutex};

use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{
    ExtensionProtocol, InstalledExtension, resolve_installed_extensions
};
use nanika_platform::companion_executable;
use nanika_search::{SearchHandle, SearchOwner, SearchSnapshot, UsageKey, UsageMap, UsageStat};
use nanika_storage::{NanikaPaths, SearchStorageWorker};

use crate::{
    BuiltInExtensionInventory, ConfigurationSaveError, ExtensionConfigurationRegistry,
    ExtensionInvocationOutcome, ExtensionRuntime, ExtensionSearchCoordinator, HostServiceHandler,
    HostServiceRouter, LifecycleError, LifecycleFailure, OperationError,
    OperationErrorCode as Code, RuntimeOutputUpdate, RuntimeUpdateBatch, RuntimeViewCompletion
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
    startup_diagnostics: Vec<crate::RuntimeWarning>,
    _notifier: crate::ExtensionNotifier,
    _recovery_wake: std::sync::mpsc::SyncSender<()>,
    _supervisor: Mutex<Option<std::thread::JoinHandle<()>>>,
    _closing: std::sync::atomic::AtomicBool,
    _restarted: Mutex<std::collections::HashMap<String, LifecycleFailure>>
}

impl RuntimeService {
    pub fn start(
        paths: &NanikaPaths,
        built_in_manifests: &[&str],
        built_in_resources: &Path
    ) -> Result<Arc<Self>, OperationError> {
        let inventory = BuiltInExtensionInventory::parse(built_in_manifests).map_err(|error| {
            OperationError::with_detail(Code::BuiltInInventoryUnavailable, error)
        })?;
        let built_in_ids = inventory
            .extensions
            .iter()
            .map(|entry| entry.manifest.id.clone())
            .collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let config_store =
            ConfigStore::open(paths.app_data_root(), paths.config_root()).map_err(|error| {
                OperationError::with_detail(Code::HostConfigurationUnavailable, error.to_string())
            })?;
        let registry = ExtensionRegistryConfig::load(&config_store).map_err(|error| {
            OperationError::with_detail(Code::ExtensionRegistryUnavailable, error.to_string())
        })?;
        let (storage_worker, storage_state) = SearchStorageWorker::spawn(paths.host_database())
            .map_err(|error| {
                OperationError::with_detail(Code::HostStorageUnavailable, error.to_string())
            })?;
        let storage = Some(storage_worker);
        diagnostics.extend(storage_state.extension_errors.into_iter().map(|detail| {
            crate::RuntimeWarning::new(
                None,
                OperationError::with_detail(Code::ExtensionStorageUnavailable, detail)
            )
        }));
        let usage = storage_state
            .usage
            .into_iter()
            .map(|stored| {
                (
                    UsageKey::new(
                        &stored.extension_id,
                        &stored.entry_id,
                        &stored.action_id,
                        &stored.query_context
                    ),
                    UsageStat {
                        execution_count: stored.execution_count,
                        last_executed_at: stored.last_executed_at
                    }
                )
            })
            .collect::<UsageMap>();
        let owner = SearchOwner::spawn(usage).map_err(|error| {
            OperationError::with_detail(Code::SearchUnavailable, error.to_string())
        })?;
        let search = owner.handle();
        if let Some(storage) = &storage {
            storage.attach_search(search.clone());
        }

        let (router, service_errors) = HostServiceRouter::spawn(paths.app_data_root());
        diagnostics.extend(service_errors.into_iter().map(|detail| {
            crate::RuntimeWarning::new(
                None,
                OperationError::with_detail(Code::HostServicesUnavailable, detail)
            )
        }));
        let router = Arc::new(router);
        let extensions = ExtensionSearchCoordinator::new();
        extensions.set_host_services(Arc::clone(&router) as Arc<dyn HostServiceHandler>);
        let configurations = Arc::new(ExtensionConfigurationRegistry::new(config_store.clone()));

        let current_executable = std::env::current_exe().map_err(|error| {
            OperationError::with_detail(Code::RuntimeStartupFailed, error.to_string())
        })?;
        let mut installed_extensions = Vec::new();
        for extension in inventory.extensions {
            let manifest = extension.manifest;
            let program = companion_executable(&current_executable, &extension.binary_name);
            if !program.is_file() {
                diagnostics.push(crate::RuntimeWarning::new(
                    Some(manifest.id.clone()),
                    OperationError::with_detail(
                        Code::ExtensionPackageUnavailable,
                        format!(
                            "built-in extension executable is missing: {}",
                            program.display()
                        )
                    )
                ));
                continue;
            }
            let resource_root = built_in_resources.join(&manifest.id);
            let extension_id = manifest.id.clone();
            let installed =
                match InstalledExtension::from_manifest(manifest, program, resource_root, true) {
                    Ok(installed) => installed,
                    Err(error) => {
                        diagnostics.push(crate::RuntimeWarning::new(
                            Some(extension_id),
                            OperationError::with_detail(
                                Code::ExtensionPackageUnavailable,
                                error.to_string()
                            )
                        ));
                        continue;
                    }
                };
            if let Some(storage) = &storage {
                storage
                    .register_builtin_extension(&installed.extension_id)
                    .and_then(nanika_storage::StorageCommit::wait)
                    .map_err(|error| {
                        OperationError::with_detail(
                            Code::HostStorageUnavailable,
                            format!(
                                "extension {} metadata could not be recorded: {error}",
                                installed.extension_id
                            )
                        )
                    })?;
            }
            installed_extensions.push(installed);
        }

        let (mut external, errors) =
            resolve_installed_extensions(paths, &storage_state.extensions, &built_in_ids);
        diagnostics.extend(errors.into_iter().map(|error| {
            crate::RuntimeWarning::new(
                None,
                OperationError::with_detail(Code::ExtensionPackageUnavailable, error.message)
            )
        }));
        installed_extensions.append(&mut external);
        let installed = installed_extensions
            .iter()
            .map(|extension| (extension.extension_id.clone(), extension.clone()))
            .collect();
        let mut extension_info = Vec::new();
        for extension in installed_extensions {
            let enabled = registry.is_enabled(&extension.extension_id);
            let configuration = configurations
                .register(
                    &extension.extension_id,
                    extension.contributes.configuration.as_ref()
                )
                .map_err(|detail| {
                    OperationError::with_detail(Code::ExtensionConfigurationUnavailable, detail)
                });
            // Fingerprint once on the startup worker, independently of extension activation.
            let icon_hash = match nanika_platform::read_png_resource(
                &extension.resource_root.join(&extension.icon),
                &extension.resource_root
            ) {
                Ok(bytes) => Some(nanika_platform::png_resource_hash(&bytes)),
                Err(error) => {
                    diagnostics.push(crate::RuntimeWarning::new(
                        Some(extension.extension_id.clone()),
                        OperationError::with_detail(
                            Code::ExtensionIconUnavailable,
                            error.to_string()
                        )
                    ));
                    None
                }
            };
            // Installed metadata remains available without a live worker or schema.
            extension_info.push(crate::RuntimeExtensionInfo {
                resource_scope: extension.localization.as_ref().map(|resources| {
                    crate::ExtensionResourceScope {
                        extension_id: extension.extension_id.clone(),
                        identity: resources.prepared().identity.clone()
                    }
                }),
                id: extension.extension_id.clone(),
                name: extension.name,
                name_search_values: extension.name_search_values,
                configuration_search_values: extension.configuration_search_values,
                icon: extension.icon,
                icon_hash,
                enabled,
                pending: false,
                state: if enabled {
                    crate::RuntimeExtensionState::Starting
                } else {
                    crate::RuntimeExtensionState::Disabled
                },
                instance_id: None,
                lifecycle_error: None,
                configuration_error: configuration.as_ref().err().cloned()
            });
            if let Err(error) = configuration {
                diagnostics.push(crate::RuntimeWarning::new(
                    Some(extension.extension_id.clone()),
                    error
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
            _restarted: Mutex::new(std::collections::HashMap::new())
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
                    .lifecycle_error = Some(error.clone().into());
                runtime.startup_diagnostics.push(crate::RuntimeWarning::new(
                    Some(info.id.clone()),
                    error.error
                ));
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
            .map_err(|error| {
                OperationError::with_detail(Code::RuntimeStartupFailed, error.to_string())
            })?;
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
    pub fn refresh_root_search(&self, generation: u64) -> Result<(), Vec<crate::RuntimeWarning>> {
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
        invocation: nanika_protocol::ActionInvocation
    ) -> Result<
        std::sync::mpsc::Receiver<Result<ExtensionInvocationOutcome, OperationError>>,
        OperationError
    > {
        if !self.search.is_current(snapshot.authority()) {
            return Err(OperationError::new(Code::SearchChanged));
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
            .ok_or_else(|| OperationError::new(Code::ActionUnavailable))?;
        let instance_id = *snapshot
            .instances
            .get(extension_id)
            .ok_or_else(|| OperationError::new(Code::SearchChanged))?;
        if self.instance_id(extension_id) != Some(instance_id) {
            return Err(OperationError::new(Code::SearchChanged));
        }
        if !self.search.is_current(snapshot.authority()) {
            return Err(OperationError::new(Code::SearchChanged));
        }
        self.extensions.invoke(
            extension_id,
            instance_id,
            snapshot.generation,
            candidate.entry_id(),
            action_id,
            query_context
        )
    }

    /// Admit a declared Settings action against the exact instance shown by Settings.
    pub fn invoke_settings_action(
        &self,
        extension_id: &str,
        instance_id: u64,
        action_id: &str,
        invocation: nanika_protocol::ActionInvocation
    ) -> Result<(), OperationError> {
        let _operation = self.configurations.operations.reserve(extension_id)?;
        let extension = self
            .installed
            .get(extension_id)
            .ok_or_else(|| OperationError::new(Code::ExtensionUnavailable))?;
        let action = extension
            .contributes
            .configuration
            .as_ref()
            .and_then(|configuration| {
                configuration
                    .actions
                    .iter()
                    .find(|item| item.action.id == action_id)
            })
            .ok_or_else(|| OperationError::new(Code::ActionUnavailable))?;
        if invocation == nanika_protocol::ActionInvocation::Default
            || !action.action.allows_invocation(invocation)
        {
            return Err(OperationError::new(Code::ActionUnavailable));
        }
        if self.instance_id(extension_id) != Some(instance_id) {
            return Err(OperationError::new(Code::ActionUnavailable));
        }
        let result = self
            .extensions
            .invoke(
                extension_id,
                instance_id,
                0,
                nanika_protocol::SETTINGS_ENTRY_ID,
                action_id,
                ""
            )?
            .recv()
            .map_err(|_| {
                OperationError::with_detail(
                    Code::InvocationUnconfirmed,
                    "Extension closed without a settings action result."
                )
            })??;
        match result {
            ExtensionInvocationOutcome::Completed {
                effect: nanika_protocol::NavigationEffect::None,
                ..
            } => Ok(()),
            ExtensionInvocationOutcome::Cancelled => Err(OperationError::new(Code::Cancelled)),
            ExtensionInvocationOutcome::Completed {
                effect:
                    nanika_protocol::NavigationEffect::Push {
                        view_id, revision, ..
                    },
                ..
            } => {
                // A rejected navigation must not leave an extension-owned view alive.
                self.close_view(extension_id, instance_id, 0, view_id, revision)
                    .and_then(|receiver| {
                        receiver.recv().map_err(|_| {
                            OperationError::with_detail(
                                Code::ViewOperationUnconfirmed,
                                "Extension closed without retiring the rejected settings view."
                            )
                        })?
                    })
                    .map_err(|cleanup| OperationError::Cleanup {
                        primary: Box::new(OperationError::new(Code::SettingsNavigationUnsupported)),
                        cleanup: Box::new(cleanup)
                    })?;
                Err(OperationError::new(Code::SettingsNavigationUnsupported))
            }
            _ => Err(OperationError::new(Code::SettingsNavigationUnsupported))
        }
    }

    /// Complete an accepted action and submit its usage to the bounded storage owner.
    /// Storage owns persistence and diagnostics; navigation never waits for its commit.
    pub fn invoke_with_usage(
        &self,
        snapshot: &Arc<SearchSnapshot>,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
        invocation: nanika_protocol::ActionInvocation
    ) -> Result<crate::RuntimeInvocationCompletion, OperationError> {
        let instance_id = self
            .instance_id(extension_id)
            .ok_or_else(|| OperationError::new(Code::ExtensionUnavailable))?;
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
                invocation
            )?
            .recv()
            .map_err(|_| {
                OperationError::with_detail(
                    Code::InvocationUnconfirmed,
                    "Extension closed without an invocation result."
                )
            })??;
        if is_default
            && matches!(outcome, ExtensionInvocationOutcome::Completed { .. })
            && let Err(error) =
                self.record_execution(extension_id, entry_id, action_id, query_context)
        {
            tracing::error!(%error, "could not submit action usage");
        }
        Ok(crate::RuntimeInvocationCompletion {
            instance_id,
            outcome
        })
    }

    pub fn record_execution(
        &self,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str
    ) -> Result<nanika_storage::StorageCommit, String> {
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
                unix_timestamp()
            )
        } else {
            storage.record_execution(
                nanika_search::normalize_history_key(query_context),
                query_context,
                UsageKey::new(extension_id, entry_id, action_id, query_context),
                unix_timestamp_millis(),
                unix_timestamp()
            )
        };
        result.map_err(|error| format!("could not record completed action: {error}"))
    }

    pub fn startup_diagnostics(&self) -> &[crate::RuntimeWarning] {
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

    pub fn configuration_menu(
        &self,
        extension_id: &str,
        instance_id: u64
    ) -> Result<crate::ConfigurationMenu, OperationError> {
        if self.instance_id(extension_id) != Some(instance_id) {
            return Err(OperationError::new(Code::ConfigurationChanged));
        }
        let Some(configuration) = self
            .extension_configurations()
            .into_iter()
            .find(|item| item.extension_id == extension_id)
        else {
            return Ok(crate::ConfigurationMenu::default());
        };
        let groups = configuration
            .contribution
            .quick_menu
            .iter()
            .map(|group| {
                let items = group
                    .items
                    .iter()
                    .map(|item| match item {
                        nanika_extension_package::ConfigurationMenuItem::Toggle { key } => {
                            crate::ConfigurationMenuControl::Toggle {
                                key: key.clone(),
                                title: configuration.contribution.properties[key].title.clone(),
                                checked: configuration.values[key]
                                    .as_bool()
                                    .expect("validated boolean setting")
                            }
                        }
                        nanika_extension_package::ConfigurationMenuItem::Action { id } => {
                            crate::ConfigurationMenuControl::Action {
                                action: configuration
                                    .contribution
                                    .actions
                                    .iter()
                                    .find(|item| item.action.id == *id)
                                    .expect("validated menu action")
                                    .action
                                    .clone()
                            }
                        }
                    })
                    .collect();
                crate::ConfigurationMenuSection {
                    id: group.id.clone(),
                    title: group.title.clone(),
                    items
                }
            })
            .collect();
        Ok(crate::ConfigurationMenu {
            revision: configuration.revision,
            groups
        })
    }

    pub fn invoke_configuration_menu_action(
        &self,
        extension_id: &str,
        instance_id: u64,
        action_id: &str,
        invocation: nanika_protocol::ActionInvocation
    ) -> Result<(), OperationError> {
        if !self
            .installed
            .get(extension_id)
            .and_then(|extension| extension.contributes.configuration.as_ref())
            .is_some_and(|configuration| {
                configuration.exposes_menu_item(
                    &nanika_extension_package::ConfigurationMenuItem::Action {
                        id: action_id.into()
                    }
                )
            })
        {
            return Err(OperationError::with_detail(
                Code::ActionUnavailable,
                "This action is not exposed in the quick menu."
            ));
        }
        self.invoke_settings_action(extension_id, instance_id, action_id, invocation)
    }

    /// Installed presentation metadata remains available without a live process.
    pub fn extension_icon(&self, extension_id: &str) -> Option<nanika_protocol::IconSource> {
        self.installed
            .get(extension_id)
            .map(|extension| nanika_protocol::IconSource::Package {
                path: extension.icon.clone()
            })
    }

    pub fn extension_resource_roots(
        &self
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
                // An admitted lifecycle operation owns its phase until it publishes completion.
                if info.enabled
                    && !info.pending
                    && !matches!(info.state, State::Stopping | State::Failed)
                {
                    let (state, error) = worker.lifecycle();
                    info.state = state;
                    info.lifecycle_error = error
                        .map(LifecycleError::from)
                        .or_else(|| info.lifecycle_error.take());
                }
            } else if info.enabled && !info.pending {
                info.state = State::Failed;
                if info.lifecycle_error.is_none() {
                    info.lifecycle_error = Some(
                        LifecycleFailure::from(
                            info.configuration_error
                                .clone()
                                .unwrap_or_else(|| OperationError::new(Code::ExtensionUnavailable))
                        )
                        .into()
                    );
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

    /// Return retained, validated content only for the admitted package and optional live instance.
    /// Settings can read installed metadata without activating a disabled extension.
    /// The desktop caller additionally authorizes the requesting window session.
    pub fn localization_resources(
        &self,
        scope: &crate::ExtensionResourceScope,
        instance_id: Option<u64>
    ) -> Result<Arc<nanika_localization::PreparedResources>, OperationError> {
        scope.validate().map_err(|detail| {
            OperationError::with_detail(Code::LocalizationScopeInvalid, detail)
        })?;
        let resources = self
            .installed
            .get(&scope.extension_id)
            .and_then(|extension| extension.localization.as_ref())
            .ok_or_else(|| OperationError::new(Code::LocalizationResourcesUnavailable))?
            .prepared();
        if resources.identity != scope.identity {
            return Err(OperationError::new(Code::LocalizationPackageChanged));
        }
        match instance_id {
            Some(instance_id) => self
                .with_extension_instance(&scope.extension_id, instance_id, || resources)
                .ok_or_else(|| OperationError::new(Code::LocalizationInstanceRetired)),
            None => Ok(resources)
        }
    }

    /// Bind search presentation to the admitted package rather than a publisher-supplied owner.
    pub fn localization_scope(&self, extension_id: &str) -> Option<crate::ExtensionResourceScope> {
        let resources = self.installed.get(extension_id)?.localization.as_ref()?;
        Some(crate::ExtensionResourceScope {
            extension_id: extension_id.to_owned(),
            identity: resources.prepared().identity.clone()
        })
    }

    /// Execute a short publication while its originating instance remains admitted.
    /// The callback must not wait for extension work or call back into this instance.
    pub fn with_extension_instance<T>(
        &self,
        extension_id: &str,
        instance_id: u64,
        publish: impl FnOnce() -> T
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
        enabled: bool
    ) -> Result<std::sync::mpsc::Receiver<Result<(), LifecycleFailure>>, LifecycleFailure> {
        if !self.installed.contains_key(extension_id) {
            return Err(LifecycleFailure::new(Code::ExtensionUnavailable));
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
                        info.lifecycle_error = Some(error.clone().into());
                        if info.state == crate::RuntimeExtensionState::Stopping {
                            info.state = crate::RuntimeExtensionState::Failed;
                        }
                    }
                }
                drop(_reservation);
                runtime.extensions.notify();
                let _ = complete.send(result);
            })
            .map_err(|error| {
                LifecycleFailure::from(OperationError::with_detail(
                    Code::ExtensionLifecycleFailed,
                    error.to_string()
                ))
            })?;
        Ok(receipt)
    }

    pub fn search_warnings(&self) -> Vec<crate::RuntimeWarning> {
        let mut warnings = self.startup_diagnostics.clone();
        warnings.extend(self.extensions.warnings());
        warnings
    }

    /// Historical storage diagnostics do not define runtime or search readiness.
    pub fn storage_failure(&self) -> Option<nanika_storage::SearchStorageFailure> {
        self.storage
            .as_ref()
            .and_then(SearchStorageWorker::last_failure)
    }

    pub fn read_text(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: String,
        read: nanika_protocol::TextRead
    ) -> Result<
        std::sync::mpsc::Receiver<Result<nanika_protocol::TextChunk, crate::OperationError>>,
        crate::OperationError
    > {
        read.validate().map_err(|detail| {
            crate::OperationError::with_detail(crate::OperationErrorCode::InvalidTextRead, detail)
        })?;
        self.extensions
            .read_text(extension_id, instance_id, generation, view_id, read)
    }

    pub fn view_event(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent
    ) -> Result<
        std::sync::mpsc::Receiver<Result<RuntimeViewCompletion, crate::OperationError>>,
        OperationError
    > {
        self.extensions.view_event(
            extension_id,
            instance_id,
            generation,
            view_id,
            revision,
            event
        )
    }

    pub fn close_view(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64
    ) -> Result<
        std::sync::mpsc::Receiver<Result<RuntimeViewCompletion, crate::OperationError>>,
        OperationError
    > {
        self.extensions
            .close_view(extension_id, instance_id, generation, view_id, revision)
    }

    /// Admit one property operation per extension. The runtime owns its completion,
    /// including persistence after application when required by the property contract.
    pub fn save_configuration(
        &self,
        extension_id: &str,
        request_id: impl Into<String>,
        key: String,
        value: serde_json::Value,
        progress: crate::ConfigurationProgressHandler
    ) -> Result<crate::ConfigurationSaveReceipt, ConfigurationSaveError> {
        let operation = self
            .configurations
            .prepare(extension_id, key, value)
            .map_err(ConfigurationSaveError::rejected)?;
        self._submit_configuration(operation, request_id, progress)
    }

    pub fn save_menu_configuration(
        &self,
        extension_id: &str,
        instance_id: u64,
        revision: u64,
        key: String,
        value: bool,
        progress: crate::ConfigurationProgressHandler
    ) -> Result<crate::ConfigurationSaveReceipt, ConfigurationSaveError> {
        // Reserve before checking instance/revision so lifecycle and Settings cannot race admission.
        let operation = self
            .configurations
            .prepare_menu(extension_id, key, value, revision)
            .map_err(ConfigurationSaveError::rejected)?;
        if self.instance_id(extension_id) != Some(instance_id) {
            return Err(ConfigurationSaveError::rejected(OperationError::new(
                Code::ConfigurationChanged
            )));
        }
        self._submit_configuration(
            operation,
            format!("menu-{instance_id}-{revision}"),
            progress
        )
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
                text: update.text
            })
            .collect();
        RuntimeUpdateBatch { outputs }
    }
    fn _submit_configuration(
        &self,
        operation: crate::ConfigurationOperation,
        request_id: impl Into<String>,
        progress: crate::ConfigurationProgressHandler
    ) -> Result<crate::ConfigurationSaveReceipt, ConfigurationSaveError> {
        let extensions = Arc::clone(&self.extensions);
        let extension_id = operation.extension_id.clone();
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
                        applied
                    ) {
                        Ok(true) => application.recv().map_err(|_| {
                            OperationError::with_detail(
                                Code::ConfigurationUnconfirmed,
                                "Extension closed without a configuration result."
                            )
                        })?,
                        Ok(false) => Ok(crate::ConfigurationApplication::Deferred),
                        Err(error) => Err(error)
                    }
                });
                extensions.notify();
                let _ = completion.send(outcome);
            })
            .map_err(|error| {
                ConfigurationSaveError::rejected(OperationError::with_detail(
                    Code::ConfigurationSubmissionFailed,
                    error.to_string()
                ))
            })?;
        Ok(crate::ConfigurationSaveReceipt(received))
    }
    fn _set_extension_enabled(
        &self,
        extension_id: &str,
        enabled: bool
    ) -> Result<(), LifecycleFailure> {
        use crate::RuntimeExtensionState as State;
        if enabled && let Some(worker) = self.extensions.worker(extension_id) {
            if worker.instance.is_active() {
                return Ok(());
            }
            return Err(LifecycleFailure::new(Code::ExtensionStillStopping));
        }
        {
            let mut transaction = nanika_config::ExtensionRegistryTransaction::begin(
                &self.config_store
            )
            .map_err(|detail| {
                LifecycleFailure::from(OperationError::with_detail(
                    Code::ExtensionRegistrySaveFailed,
                    detail
                ))
            })?;
            transaction.set_enabled(extension_id, enabled);
            transaction.save().map_err(|detail| {
                LifecycleFailure::from(OperationError::with_detail(
                    Code::ExtensionRegistrySaveFailed,
                    detail
                ))
            })?;
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
            let cause = worker
                .lifecycle()
                .1
                .unwrap_or_else(|| LifecycleFailure::new(Code::ExtensionUnavailable));
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
                    LifecycleError::Restarting {
                        failure: Box::new(cause.clone())
                    }
                } else {
                    LifecycleError::Stopping {
                        failure: Box::new(cause.clone())
                    }
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
            // A successful stop may add the drained process tail to the original failure.
            // A stop failure is a separate cause and must not replace the crash that triggered recovery.
            let cause = if stopped.is_ok() {
                worker.lifecycle().1.unwrap_or(cause)
            } else {
                cause
            };
            let previous = self
                ._restarted
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(info.id.clone(), cause.clone());
            let failures = retired
                .err()
                .into_iter()
                .chain(stopped.err())
                .collect::<Vec<_>>();
            let result = if !failures.is_empty() {
                Err(LifecycleError::RecoveryStopped {
                    original: Box::new(cause),
                    failures
                })
            } else {
                self.extensions.remove_stopped(&info.id);
                if let Some(first) = previous {
                    Err(LifecycleError::RestartLimitReached {
                        original: Box::new(first),
                        failure: Box::new(cause)
                    })
                } else if self._closing.load(std::sync::atomic::Ordering::Acquire) {
                    Err(LifecycleError::RecoveryStopped {
                        original: Box::new(cause),
                        failures: vec![LifecycleFailure::new(Code::ApplicationShuttingDown)]
                    })
                } else {
                    self._activate_extension(&info.id, true).map_err(|failure| {
                        LifecycleError::RestartFailed {
                            original: Box::new(cause),
                            failure: Box::new(failure)
                        }
                    })
                }
            };
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
                        current.lifecycle_error = Some(error);
                    }
                }
            }
            drop(reservation);
            self.extensions.notify();
        }
    }

    /// Startup and explicit enable use the same worker-owned activation path.
    fn _activate_extension(
        &self,
        extension_id: &str,
        restarting: bool
    ) -> Result<(), LifecycleFailure> {
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
                .register(extension_id, descriptor.contributes.configuration.as_ref())
                .map_err(|detail| {
                    OperationError::with_detail(Code::ExtensionConfigurationUnavailable, detail)
                });
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
        let source = crate::ExtensionRuntimeSource {
            localization: descriptor.localization.clone(),
            activation: if restarting {
                nanika_extension_package::ExtensionActivation::Startup
            } else {
                descriptor.activation
            },
            live_configuration: matches!(descriptor.protocol, ExtensionProtocol::Nanika { .. }),
            start: Box::new(move |configuration| {
                let runtime = spawn_runtime(
                    &owned_descriptor.extension_id,
                    owned_descriptor.protocol,
                    &owned_descriptor.program,
                    &paths,
                    configuration
                )?;
                Ok(runtime)
            })
        };
        self.extensions
            .register_source(
                extension_id,
                source,
                self.search.clone(),
                descriptor.contributes.clone(),
                configuration
            )
            .map_err(|error| LifecycleFailure::from(crate::SupervisorError::Io(error)))?;
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
    configuration: nanika_protocol::ExtensionConfiguration
) -> Result<ExtensionRuntime, std::io::Error> {
    ExtensionRuntime::spawn_with_configuration(
        extension_id,
        protocol,
        program,
        extension_arguments(protocol, paths),
        Default::default(),
        configuration
    )
}

fn extension_arguments(protocol: ExtensionProtocol, paths: &NanikaPaths) -> Vec<OsString> {
    match protocol {
        ExtensionProtocol::Nanika {
            protocol_version: 1
        } => vec![
            path_argument("data-root", paths.app_data_root()),
            path_argument("cache-root", paths.cache_root()),
        ],
        _ => Vec::new()
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
