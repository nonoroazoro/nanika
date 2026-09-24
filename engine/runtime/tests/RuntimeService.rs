use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nanika_host::RuntimeService;
use nanika_storage::NanikaPaths;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

const WAIT: Duration = Duration::from_secs(5);
const HEALTHY: &str = "com.nanika.application";
const DELAYED: &str = "com.nanika.script";

struct Fixture {
    paths: NanikaPaths,
    binary: PathBuf,
    manifests: Vec<String>,
}

impl Fixture {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "nanika-runtime-fixture-{}-{unique}-{sequence}",
            std::process::id()
        );
        let root = std::env::temp_dir().join(&name);
        std::fs::create_dir_all(&root).unwrap();
        let binary = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(env!("CARGO_BIN_EXE_nanika-extension-fixture"), &binary).unwrap();
        let target = nanika_platform::target_triple();
        let manifests = [HEALTHY, DELAYED].map(|id| {
            serde_json::json!({
                "format": "nanika-extension",
                "name": "Test Extension", "icon": "extension",
        "manifestVersion": 1,
                "id": id,
                "version": "0.1.0",
                "hostApi": "^0.1",
                "targets": {
                    target: {
                        "entrypoint": format!("bin/{target}/{name}{}", std::env::consts::EXE_SUFFIX)
                    }
                },
                "runtime": { "protocol": "nanika", "protocolVersion": 1 },
                "contributes": {
                    "rootSearch": {},
                    "configuration": {
                        "title": "Fixture",
                        "properties": {
                            "fixture.enabled": {
                                "type": "boolean",
                                "title": "Enabled",
                                "default": true
                            }
                        }
                    }
                }
            })
            .to_string()
        });
        Self {
            paths: NanikaPaths::from_roots(&root, root.join("cache"), root.join("config")),
            binary,
            manifests: manifests.into(),
        }
    }

    fn block(&self, operation: &str) {
        std::fs::write(
            self.paths
                .app_data_root()
                .join(format!("{operation}.block")),
            b"block",
        )
        .unwrap();
    }

    fn release(&self, operation: &str) {
        std::fs::remove_file(
            self.paths
                .app_data_root()
                .join(format!("{operation}.block")),
        )
        .unwrap();
    }

    fn entered(&self, operation: &str) -> bool {
        self.paths
            .app_data_root()
            .join(format!("{operation}.entered"))
            .exists()
    }

    fn start(&self) -> RuntimeService {
        let paths = self.paths.clone();
        let manifests = self.manifests.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let sources = manifests.iter().map(String::as_str).collect::<Vec<_>>();
            let _ = sender.send(RuntimeService::start(&paths, &sources));
        });
        let result = receiver.recv_timeout(WAIT);
        if result.is_err() {
            self.release_all();
            thread.join().unwrap();
        }
        result
            .expect("one pending extension must not block runtime startup")
            .expect("runtime starts")
    }

    fn stop(&self, runtime: RuntimeService) {
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            drop(runtime);
            let _ = sender.send(());
        });
        let result = receiver.recv_timeout(WAIT);
        // Release the fixture on regression so a failing test never strands a process.
        self.release_all();
        thread.join().unwrap();
        result.expect("shutdown must interrupt pending extension requests");
    }

    fn release_all(&self) {
        for entry in std::fs::read_dir(self.paths.app_data_root())
            .unwrap()
            .flatten()
        {
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "block")
            {
                std::fs::remove_file(entry.path()).unwrap();
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.release_all();
        std::fs::remove_dir_all(self.paths.app_data_root()).unwrap();
        std::fs::remove_file(&self.binary).unwrap();
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "fixture condition was not observed"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn has_result(runtime: &RuntimeService, generation: u64, extension_id: &str) -> bool {
    runtime.latest_snapshot().is_some_and(|snapshot| {
        snapshot.generation == generation
            && snapshot
                .results
                .iter()
                .any(|result| result.candidate.extension_id() == extension_id)
    })
}

#[test]
fn pending_initialization_is_isolated_and_late_worker_uses_latest_query() {
    let fixture = Fixture::new();
    let operation = format!("initialize-{DELAYED}");
    fixture.block(&operation);
    let runtime = fixture.start();
    wait_until(|| fixture.entered(&operation));
    let first = runtime.begin_query("first").unwrap();
    wait_until(|| has_result(&runtime, first, HEALTHY));
    assert!(!has_result(&runtime, first, DELAYED));

    let latest = runtime.begin_query("latest").unwrap();
    wait_until(|| has_result(&runtime, latest, HEALTHY));
    fixture.release(&operation);
    wait_until(|| has_result(&runtime, latest, DELAYED));
    assert!(
        runtime
            .latest_snapshot()
            .unwrap()
            .results
            .iter()
            .all(|result| result.candidate.title() == "latest")
    );
    fixture.stop(runtime);
}

#[test]
fn query_failure_is_a_local_warning_and_healthy_results_remain_usable() {
    let fixture = Fixture::new();
    let runtime = fixture.start();
    let first = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, first, HEALTHY) && has_result(&runtime, first, DELAYED));
    std::fs::write(
        fixture
            .paths
            .app_data_root()
            .join(format!("fail-search-{DELAYED}")),
        b"fail",
    )
    .unwrap();
    let generation = runtime.begin_query("failure").unwrap();
    wait_until(|| {
        has_result(&runtime, generation, HEALTHY)
            && runtime
                .search_warnings()
                .iter()
                .any(|warning| warning.contains(DELAYED))
    });
    assert!(runtime.active_error().is_none());
    assert!(!has_result(&runtime, generation, DELAYED));
    assert!(
        runtime
            .search_warnings()
            .iter()
            .all(|warning| !warning.contains("internal cause"))
    );
    runtime
        .invoke(
            &runtime.latest_snapshot().unwrap(),
            HEALTHY,
            "fixture.entry",
            "fixture.run",
            "failure",
            nanika_protocol::ActionInvocation::Default,
        )
        .unwrap()
        .recv_timeout(WAIT)
        .unwrap()
        .unwrap();
    fixture.stop(runtime);
}

#[test]
fn shutdown_interrupts_initialization() {
    let fixture = Fixture::new();
    let operation = format!("initialize-{DELAYED}");
    fixture.block(&operation);
    let runtime = fixture.start();
    wait_until(|| fixture.entered(&operation));
    fixture.stop(runtime);
}

#[test]
fn shutdown_interrupts_a_pending_configuration_update() {
    let fixture = Fixture::new();
    let runtime = fixture.start();
    let generation = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, generation, DELAYED));
    fixture.block("update-configuration");
    runtime
        .update_configuration(
            DELAYED,
            "update-configuration",
            std::collections::BTreeMap::from([(
                "fixture.enabled".to_owned(),
                serde_json::json!(false),
            )]),
        )
        .unwrap();
    wait_until(|| fixture.entered("update-configuration"));
    fixture.stop(runtime);
}

#[test]
fn live_configuration_updates_report_the_correlated_acknowledgement() {
    let fixture = Fixture::new();
    let runtime = fixture.start();
    let generation = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    let disposition = runtime
        .update_configuration(
            HEALTHY,
            "update-configuration-success",
            std::collections::BTreeMap::from([(
                "fixture.enabled".to_owned(),
                serde_json::json!(false),
            )]),
        )
        .unwrap();
    assert_eq!(
        disposition,
        nanika_host::ConfigurationUpdateDisposition::LiveApplyQueued
    );

    let mut acknowledgement = None;
    wait_until(|| {
        acknowledgement = runtime
            .take_updates()
            .configurations
            .into_iter()
            .find(|update| update.request_id == "update-configuration-success");
        acknowledgement.is_some()
    });
    let acknowledgement = acknowledgement.expect("configuration acknowledgement");
    assert_eq!(acknowledgement.extension_id, HEALTHY);
    assert_eq!(acknowledgement.result, Ok(()));
    fixture.stop(runtime);
}

#[test]
fn zero_extension_host_is_ready_without_an_initialization_barrier() {
    let mut fixture = Fixture::new();
    fixture.manifests.clear();
    let runtime = fixture.start();
    let generation = runtime.begin_query("empty").unwrap();
    wait_until(|| {
        runtime
            .latest_snapshot()
            .is_some_and(|snapshot| snapshot.generation == generation)
    });
    assert!(runtime.latest_snapshot().unwrap().results.is_empty());
    fixture.stop(runtime);
}

#[test]
fn queries_continue_while_configuration_application_is_pending() {
    let fixture = Fixture::new();
    let runtime = fixture.start();
    let generation = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    let receipt = runtime
        .save_configuration(
            HEALTHY,
            "deferred-settings",
            std::collections::BTreeMap::from([(
                "fixture.enabled".to_owned(),
                serde_json::json!(false),
            )]),
        )
        .unwrap();
    wait_until(|| fixture.entered("deferred-settings"));
    // The fixture withholds ConfigurationApplied until it can service the next query.
    let generation = runtime.begin_query("after save").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    assert_eq!(
        receipt.wait(),
        nanika_host::ConfigurationSaveOutcome::Applied
    );
    fixture.stop(runtime);
}

#[test]
fn settings_save_returns_before_application_and_keeps_configuration_readable() {
    let fixture = Fixture::new();
    let runtime = std::sync::Arc::new(fixture.start());
    let generation = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    fixture.block("settings-save");
    let owner = std::sync::Arc::clone(&runtime);
    let (sent, received) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        sent.send(owner.save_configuration(
            HEALTHY,
            "settings-save",
            std::collections::BTreeMap::from([(
                "fixture.enabled".to_owned(),
                serde_json::json!(false),
            )]),
        ))
        .unwrap();
    });
    wait_until(|| fixture.entered("settings-save"));
    let receipt = received.recv_timeout(WAIT).unwrap().unwrap();
    assert!(matches!(
        receipt,
        nanika_host::ConfigurationSaveReceipt::Pending(_)
    ));
    thread.join().unwrap();
    assert_eq!(
        runtime
            .extension_configurations()
            .into_iter()
            .find(|entry| entry.extension_id == HEALTHY)
            .unwrap()
            .values["fixture.enabled"],
        false
    );
    fixture.release("settings-save");
    assert_eq!(
        receipt.wait(),
        nanika_host::ConfigurationSaveOutcome::Applied
    );
    assert!(
        runtime.take_updates().configurations.is_empty(),
        "a save has exactly one completion consumer"
    );
    runtime.shutdown();
}

#[test]
fn settings_distinguishes_validation_failure_from_saved_application_failure() {
    let fixture = Fixture::new();
    let runtime = fixture.start();
    let generation = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    std::fs::write(
        fixture.paths.app_data_root().join("fail-settings-rejected"),
        b"reject",
    )
    .unwrap();
    let outcome = runtime
        .save_configuration(
            HEALTHY,
            "settings-rejected",
            std::collections::BTreeMap::from([(
                "fixture.enabled".to_owned(),
                serde_json::json!(false),
            )]),
        )
        .unwrap()
        .wait();
    assert!(
        matches!(outcome, nanika_host::ConfigurationSaveOutcome::ApplyFailed(error) if error.contains("fixture could not apply configuration"))
    );
    let invalid = runtime.save_configuration(
        HEALTHY,
        "settings-invalid",
        std::collections::BTreeMap::from([(
            "fixture.enabled".to_owned(),
            serde_json::json!("invalid"),
        )]),
    );
    assert!(invalid.is_err());
    assert_eq!(
        runtime
            .extension_configurations()
            .into_iter()
            .find(|entry| entry.extension_id == HEALTHY)
            .unwrap()
            .values["fixture.enabled"],
        false
    );
    fixture.stop(runtime);
    let reopened = fixture.start();
    assert_eq!(
        reopened
            .extension_configurations()
            .into_iter()
            .find(|entry| entry.extension_id == HEALTHY)
            .unwrap()
            .values["fixture.enabled"],
        false
    );
    fixture.stop(reopened);
}

#[test]
fn shutdown_completes_a_waiting_settings_save() {
    let fixture = Fixture::new();
    let runtime = std::sync::Arc::new(fixture.start());
    let generation = runtime.begin_query("ready").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    fixture.block("settings-pending");
    let owner = std::sync::Arc::clone(&runtime);
    let (sent, received) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        sent.send(owner.save_configuration(
            HEALTHY,
            "settings-pending",
            std::collections::BTreeMap::from([(
                "fixture.enabled".to_owned(),
                serde_json::json!(false),
            )]),
        ))
        .unwrap();
    });
    wait_until(|| fixture.entered("settings-pending"));
    runtime.shutdown();
    assert!(matches!(
        received.recv_timeout(WAIT).unwrap().unwrap().wait(),
        nanika_host::ConfigurationSaveOutcome::ApplyFailed(_)
    ));
    thread.join().unwrap();
}

#[test]
fn static_catalog_does_not_activate_on_demand_processes_and_success_is_recorded_without_ui() {
    for activation in ["startup", "onDemand"] {
        let mut fixture = Fixture::new();
        fixture.manifests.truncate(1);
        let mut manifest: serde_json::Value = serde_json::from_str(&fixture.manifests[0]).unwrap();
        manifest["activation"] = activation.into();
        manifest["contributes"]
            .as_object_mut()
            .unwrap()
            .remove("rootSearch");
        manifest["contributes"]["commands"] = serde_json::json!([{
            "command": "fixture.entry",
            "action": nanika_protocol::Action::primary(nanika_protocol::COMMAND_EXECUTE_ACTION_ID, "Run"), "title": "Fixture command", "description": "A static command"
        }]);
        fixture.manifests[0] = manifest.to_string();
        let initializing = format!("initialize-{HEALTHY}");
        let runtime = fixture.start();
        let info = runtime.extension_info();
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].id, HEALTHY);
        assert_eq!(info[0].name, "Test Extension");
        assert_eq!(serde_json::to_value(&info[0]).unwrap()["icon"], "extension");
        let generation = runtime.begin_query("fixture").unwrap();
        wait_until(|| has_result(&runtime, generation, HEALTHY));
        runtime.prepare_visible_entries(&runtime.latest_snapshot().unwrap(), 10);
        if activation == "startup" {
            wait_until(|| fixture.entered(&initializing));
        } else {
            // A configuration acknowledgement fences the dormant worker after its
            // preparation hint, proving neither event started the process.
            runtime
                .update_configuration(
                    HEALTHY,
                    "dormant-config",
                    std::collections::BTreeMap::from([(
                        "fixture.enabled".to_owned(),
                        serde_json::json!(false),
                    )]),
                )
                .unwrap();
            wait_until(|| !runtime.take_updates().configurations.is_empty());
            assert!(!fixture.entered(&initializing));
        }
        println!(
            "activation={activation}, initialized_before_invoke={}",
            usize::from(fixture.entered(&initializing))
        );
        let completion = runtime
            .invoke_recorded(
                &runtime.latest_snapshot().unwrap(),
                HEALTHY,
                "fixture.entry",
                nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                "fixture",
                nanika_protocol::ActionInvocation::Default,
            )
            .unwrap();
        assert!(matches!(
            completion.outcome,
            nanika_host::ExtensionInvocationOutcome::Completed { .. }
        ));
        assert!(completion.recording_error.is_none());
        assert!(fixture.entered(&initializing));
        // Explicit shutdown must work while another owner still holds the runtime.
        let runtime = std::sync::Arc::new(runtime);
        let other_owner = std::sync::Arc::clone(&runtime);
        runtime.shutdown();
        assert!(
            runtime
                .invoke(
                    &runtime.latest_snapshot().unwrap(),
                    HEALTHY,
                    "fixture.entry",
                    nanika_protocol::COMMAND_EXECUTE_ACTION_ID,
                    "fixture",
                    nanika_protocol::ActionInvocation::Default,
                )
                .is_err()
        );
        let (storage, stored) =
            nanika_storage::SearchStorageWorker::spawn(fixture.paths.host_database()).unwrap();
        assert_eq!(stored.usage.len(), 1);
        assert_eq!(stored.usage[0].execution_count, 1);
        assert_eq!(stored.input_history, ["fixture"]);
        storage.shutdown();
        drop(other_owner);
        drop(runtime);
    }
}

#[test]
fn explicit_shutdown_interrupts_initialization_with_a_retained_runtime_owner() {
    let fixture = Fixture::new();
    let operation = format!("initialize-{DELAYED}");
    fixture.block(&operation);
    let runtime = std::sync::Arc::new(fixture.start());
    wait_until(|| fixture.entered(&operation));
    let owner = std::sync::Arc::clone(&runtime);
    let (sent, received) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        owner.shutdown();
        sent.send(()).unwrap();
    });
    if received.recv_timeout(WAIT).is_err() {
        fixture.release_all();
        panic!("explicit shutdown must not depend on dropping the last Arc");
    }
    thread.join().unwrap();
    drop(runtime);
}

#[test]
fn recording_failure_preserves_the_completed_view_and_its_close_contract() {
    let mut fixture = Fixture::new();
    fixture.manifests.truncate(1);
    let mut manifest: serde_json::Value = serde_json::from_str(&fixture.manifests[0]).unwrap();
    manifest["contributes"]
        .as_object_mut()
        .unwrap()
        .remove("rootSearch");
    manifest["contributes"]["views"] = serde_json::json!([{
        "id": "fixture.view", "title": "Fixture view", "description": "A static view"
    }]);
    fixture.manifests[0] = manifest.to_string();
    let runtime = fixture.start();
    let generation = runtime.begin_query("fixture").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));

    // Fail the real storage transaction after the extension succeeds, without
    // changing permissions or replacing a live database on either platform.
    let database = rusqlite::Connection::open(fixture.paths.host_database()).unwrap();
    database
        .execute_batch(
            "CREATE TRIGGER reject_usage BEFORE INSERT ON usage_stats
         BEGIN SELECT RAISE(ABORT, 'fixture recording failure'); END;",
        )
        .unwrap();
    let completion = runtime
        .invoke_recorded(
            &runtime.latest_snapshot().unwrap(),
            HEALTHY,
            "fixture.view",
            nanika_protocol::VIEW_OPEN_ACTION_ID,
            "fixture",
            nanika_protocol::ActionInvocation::Default,
        )
        .unwrap();
    assert!(
        completion
            .recording_error
            .unwrap()
            .contains("fixture recording failure")
    );
    let nanika_host::ExtensionInvocationOutcome::Completed {
        effect:
            nanika_protocol::NavigationEffect::Push {
                view_id, revision, ..
            },
        ..
    } = completion.outcome
    else {
        panic!("recording failure must preserve the created view");
    };
    runtime
        .close_view(HEALTHY, generation, &view_id, revision)
        .unwrap()
        .recv_timeout(WAIT)
        .unwrap()
        .unwrap();
    assert!(
        runtime
            .close_view(HEALTHY, generation, &view_id, revision)
            .unwrap()
            .recv_timeout(WAIT)
            .unwrap()
            .is_err(),
        "the first close must release the live view"
    );
    let stored = nanika_storage::HostDatabase::open(fixture.paths.host_database()).unwrap();
    assert!(stored.load_usage().unwrap().is_empty());
    assert!(
        stored.load_input_history().unwrap().is_empty(),
        "recording must roll back atomically"
    );
    drop(stored);
    drop(database);
    fixture.stop(runtime);
}

#[test]
fn restricted_candidates_stay_searchable_but_enforce_execution_policy() {
    use nanika_protocol::ActionInvocation;
    let mut fixture = Fixture::new();
    fixture.manifests.truncate(1);
    let runtime = fixture.start();
    for (query, accepted) in [
        ("explicit-only", ActionInvocation::Explicit),
        ("confirm-action", ActionInvocation::Confirmed),
    ] {
        let generation = runtime.begin_query(query).unwrap();
        wait_until(|| has_result(&runtime, generation, HEALTHY));
        let invoke = |invocation| {
            runtime.invoke(
                &runtime.latest_snapshot().unwrap(),
                HEALTHY,
                "fixture.entry",
                "fixture.run",
                query,
                invocation,
            )
        };
        assert!(invoke(ActionInvocation::Default).is_err());
        if query == "confirm-action" {
            assert!(invoke(ActionInvocation::Explicit).is_err());
        }
        let outcome = invoke(accepted)
            .unwrap()
            .recv_timeout(WAIT)
            .unwrap()
            .unwrap();
        assert!(matches!(
            outcome,
            nanika_host::ExtensionInvocationOutcome::Completed { .. }
        ));
    }
    fixture.stop(runtime);
}

#[test]
fn static_command_preserves_confirmation_policy_through_runtime_dispatch() {
    use nanika_protocol::{Action, ActionInvocation, ActionStyle, COMMAND_EXECUTE_ACTION_ID};
    let mut fixture = Fixture::new();
    fixture.manifests.truncate(1);
    let mut manifest: serde_json::Value = serde_json::from_str(&fixture.manifests[0]).unwrap();
    let mut action = Action::primary(COMMAND_EXECUTE_ACTION_ID, "Empty");
    action.allow_default_execution = false;
    action.style = ActionStyle::Destructive;
    action.confirmation_title = Some("Empty now?".to_owned());
    manifest["contributes"]
        .as_object_mut()
        .unwrap()
        .remove("rootSearch");
    manifest["contributes"]["commands"] = serde_json::json!([{
        "command": "fixture.entry", "title": "Fixture command", "description": "A static command",
        "action": action,
    }]);
    fixture.manifests[0] = manifest.to_string();
    let runtime = fixture.start();
    let generation = runtime.begin_query("fixture").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    let snapshot = runtime.latest_snapshot().unwrap();
    let candidate = &snapshot.results[0].candidate;
    assert_eq!(candidate.actions(), &[action]);
    let invoke = |invocation| {
        runtime.invoke(
            &runtime.latest_snapshot().unwrap(),
            HEALTHY,
            "fixture.entry",
            COMMAND_EXECUTE_ACTION_ID,
            "fixture",
            invocation,
        )
    };
    assert!(invoke(ActionInvocation::Default).is_err());
    assert!(invoke(ActionInvocation::Explicit).is_err());
    assert!(matches!(
        invoke(ActionInvocation::Confirmed)
            .unwrap()
            .recv_timeout(WAIT)
            .unwrap()
            .unwrap(),
        nanika_host::ExtensionInvocationOutcome::Completed { .. }
    ));
    fixture.stop(runtime);
}

#[test]
fn confirmation_rejects_a_replaced_snapshot_within_the_same_generation() {
    use nanika_protocol::ActionInvocation;
    let fixture = Fixture::new();
    let initializing = format!("initialize-{DELAYED}");
    fixture.block(&initializing);
    let runtime = fixture.start();
    wait_until(|| fixture.entered(&initializing));
    let generation = runtime.begin_query("confirm-action").unwrap();
    wait_until(|| has_result(&runtime, generation, HEALTHY));
    let reviewed = runtime.latest_snapshot().unwrap();

    // A late contributor replaces the runtime snapshot while a WebView may still
    // be displaying the previous revision awaiting channel acknowledgement.
    fixture.release(&initializing);
    wait_until(|| has_result(&runtime, generation, DELAYED));
    let current = runtime.latest_snapshot().unwrap();
    assert_eq!(reviewed.generation, current.generation);
    assert!(!std::sync::Arc::ptr_eq(&reviewed, &current));
    let rejected = runtime.invoke_recorded(
        &reviewed,
        HEALTHY,
        "fixture.entry",
        "fixture.run",
        "confirm-action",
        ActionInvocation::Confirmed,
    );
    assert!(matches!(rejected, Err(error) if error.contains("Search changed")));
    let stored = nanika_storage::HostDatabase::open(fixture.paths.host_database()).unwrap();
    assert!(stored.load_usage().unwrap().is_empty());
    drop(stored);

    let completion = runtime
        .invoke_recorded(
            &current,
            HEALTHY,
            "fixture.entry",
            "fixture.run",
            "confirm-action",
            ActionInvocation::Confirmed,
        )
        .unwrap();
    assert!(matches!(
        completion.outcome,
        nanika_host::ExtensionInvocationOutcome::Completed { .. }
    ));
    fixture.stop(runtime);
}

#[test]
fn installed_extensions_share_disablement_configuration_and_reenable_contracts() {
    for external in [false, true] {
        for configurable in [false, true] {
            let mut fixture = Fixture::new();
            fixture.manifests.truncate(1);
            let id = if external {
                "com.example.lifecycle"
            } else {
                HEALTHY
            };
            let mut manifest: serde_json::Value =
                serde_json::from_str(&fixture.manifests[0]).unwrap();
            manifest["id"] = serde_json::json!(id);
            if !configurable {
                manifest["contributes"]
                    .as_object_mut()
                    .unwrap()
                    .remove("configuration");
            }
            fixture.manifests[0] = manifest.to_string();
            if external {
                let installed = fixture
                    .paths
                    .app_data_root()
                    .join("extensions")
                    .join(id)
                    .join("0.1.0");
                let entrypoint =
                    manifest["targets"][nanika_platform::target_triple()]["entrypoint"]
                        .as_str()
                        .unwrap();
                let program = installed.join(entrypoint);
                std::fs::create_dir_all(program.parent().unwrap()).unwrap();
                std::fs::copy(&fixture.binary, &program).unwrap();
                std::fs::write(installed.join("manifest.jsonc"), manifest.to_string()).unwrap();
                nanika_storage::HostDatabase::open(fixture.paths.host_database())
                    .unwrap()
                    .install_external_extension(id, "0.1.0", &installed, "fixture-digest", 1)
                    .unwrap();
                fixture.manifests.clear();
            }
            let store = nanika_config::ConfigStore::open(
                fixture.paths.app_data_root(),
                fixture.paths.config_root(),
            )
            .unwrap();
            let mut registry = nanika_config::ExtensionRegistryConfig::default();
            registry.set_enabled(id, false);
            registry.save(&store).unwrap();
            let initializing = format!("initialize-{id}");
            fixture.block(&initializing);
            let runtime = fixture.start();
            assert_eq!(runtime.extension_info().len(), 1);
            let info = &runtime.extension_info()[0];
            assert_eq!(info.id, id);
            assert!(!info.enabled);
            assert!(info.configuration_error.is_none());
            let generation = runtime.begin_query("fixture").unwrap();
            wait_until(|| {
                runtime
                    .latest_snapshot()
                    .is_some_and(|snapshot| snapshot.generation == generation)
            });
            assert!(runtime.latest_snapshot().unwrap().results.is_empty());
            assert!(!fixture.entered(&initializing));
            assert_eq!(
                runtime.extension_configurations().len(),
                usize::from(configurable)
            );
            if configurable {
                let receipt = runtime
                    .save_configuration(
                        id,
                        "disabled-settings",
                        std::collections::BTreeMap::from([(
                            "fixture.enabled".to_owned(),
                            serde_json::json!(false),
                        )]),
                    )
                    .unwrap();
                assert!(matches!(
                    receipt.wait(),
                    nanika_host::ConfigurationSaveOutcome::SavedForNextLaunch
                ));
            }
            fixture.stop(runtime);
            let database =
                nanika_storage::HostDatabase::open(fixture.paths.host_database()).unwrap();
            assert!(database.extension(id).unwrap().is_some());
            drop(database);
            // The public management operation must accept either provenance.
            nanika_extension_package::set_extension_enabled(id, true, &fixture.paths, &store)
                .unwrap();
            assert!(
                nanika_config::ExtensionRegistryConfig::load(&store)
                    .unwrap()
                    .is_enabled(id)
            );
            let runtime = fixture.start();
            assert!(runtime.extension_info()[0].enabled);
            let generation = runtime.begin_query("fixture").unwrap();
            wait_until(|| has_result(&runtime, generation, id));
            if configurable {
                assert_eq!(
                    runtime.extension_configurations()[0].values["fixture.enabled"],
                    serde_json::json!(false)
                );
            }
            fixture.stop(runtime);
            nanika_extension_package::set_extension_enabled(id, false, &fixture.paths, &store)
                .unwrap();
            let runtime = fixture.start();
            assert!(!runtime.extension_info()[0].enabled);
            fixture.stop(runtime);
            assert!(
                !nanika_config::ExtensionRegistryConfig::load(&store)
                    .unwrap()
                    .is_enabled(id)
            );
        }
    }
}

#[test]
fn invalid_configuration_keeps_installed_metadata_visible() {
    let mut fixture = Fixture::new();
    fixture.manifests.truncate(1);
    let store = nanika_config::ConfigStore::open(
        fixture.paths.app_data_root(),
        fixture.paths.config_root(),
    )
    .unwrap();
    let path = store.extension_configuration_file(HEALTHY);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "{ malformed").unwrap();
    let runtime = fixture.start();
    assert_eq!(runtime.extension_info().len(), 1);
    assert!(runtime.extension_info()[0].configuration_error.is_some());
    assert!(runtime.extension_configurations().is_empty());
    fixture.stop(runtime);
}
