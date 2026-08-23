use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{CancelNotification, InitializeRequest, PromptRequest, SessionId},
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client, ConnectionTo, Error};
use futures_lite::future;
use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{ExtensionProtocol, install_package, resolve_active_extensions};
use nanika_host::{
    ExtensionLimits, ExtensionRuntime, ExtensionRuntimeInvocation, ExtensionSearchCoordinator,
    SupervisorError,
};
use nanika_search::{SearchOwner, UsageMap};
use nanika_storage::{HostDatabase, NanikaPaths};
use zip::write::SimpleFileOptions;

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn negotiates_acp_v1_with_unique_sessions_and_cancellation() {
    run_with_timeout(async {
        let agent = AcpAgent::new(AcpAgentConfig::new(dummy_executable()));
        Client
            .builder()
            .connect_with(agent, |connection: ConnectionTo<Agent>| async move {
                let initialized = connection
                    .send_request(InitializeRequest::new(ProtocolVersion::from(2_u16)))
                    .block_task()
                    .await?;
                assert_eq!(initialized.protocol_version, ProtocolVersion::V1);

                let unknown_prompt = connection
                    .send_request(PromptRequest::new(
                        SessionId::new("unknown"),
                        vec!["hello".into()],
                    ))
                    .block_task()
                    .await;
                assert!(unknown_prompt.is_err());

                let mut first = connection
                    .build_session(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
                    .block_task()
                    .start_session()
                    .await?;
                let second = connection
                    .build_session(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
                    .block_task()
                    .start_session()
                    .await?;
                assert_ne!(first.session_id(), second.session_id());

                first.send_prompt("hello")?;
                assert_eq!(first.read_to_string().await?, "Hello World");
                connection.send_notification_to(
                    Agent,
                    CancelNotification::new(first.session_id().clone()),
                )?;
                Ok(())
            })
            .await
    });
}

#[test]
fn package_install_resolution_and_host_adapter_round_trip() {
    let root = temporary_root("host-adapter");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("dummy.nanika");
    create_package(&package, &dummy_executable());

    install_package(&package, &paths, &store).expect("install ACP package");
    let database = HostDatabase::open(paths.host_database()).expect("database");
    let records = database.load_extensions().expect("load extensions");
    let registry = ExtensionRegistryConfig::load(&store).expect("registry");
    let (mut active, errors) = resolve_active_extensions(&paths, &records, &registry);
    assert!(errors.is_empty(), "resolution errors: {errors:?}");
    assert_eq!(active.len(), 1);
    let extension = active.pop().expect("active ACP extension");
    assert_eq!(
        extension.protocol,
        ExtensionProtocol::Acp {
            protocol_version: 1
        }
    );

    let limits = ExtensionLimits {
        handshake_timeout: TEST_TIMEOUT,
        action_timeout: TEST_TIMEOUT,
        shutdown_timeout: TEST_TIMEOUT,
        ..ExtensionLimits::default()
    };
    let mut runtime = ExtensionRuntime::spawn_with(
        &extension.extension_id,
        extension.protocol,
        extension.program,
        std::iter::empty(),
        limits,
    )
    .expect("spawn ACP runtime");
    runtime
        .initialize("initialize-dummy")
        .expect("initialize ACP runtime");
    let candidates = Arc::new(Mutex::new(Vec::new()));
    let published_candidates = Arc::clone(&candidates);
    runtime
        .query_incremental(
            "query-dummy",
            1,
            "@com.example.acp-dummy hello",
            TEST_TIMEOUT,
            move |entries| {
                *published_candidates
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = entries;
                Ok(())
            },
            || false,
        )
        .expect("query ACP runtime");
    let candidates = candidates.lock().unwrap_or_else(|error| error.into_inner());
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].entry_id, "prompt");
    assert_eq!(candidates[0].action_id, "prompt");
    drop(candidates);
    let streamed = Arc::new(Mutex::new(String::new()));
    let published_output = Arc::clone(&streamed);
    let output = runtime
        .invoke_cancellable(
            ExtensionRuntimeInvocation::new(
                "invoke-dummy",
                1,
                "prompt",
                "prompt",
                "@com.example.acp-dummy hello",
            ),
            Arc::new(move |chunk| {
                published_output
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .push_str(&chunk);
            }),
            || false,
        )
        .expect("invoke ACP prompt");
    assert!(output);
    assert_eq!(
        streamed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_str(),
        "Hello World"
    );
    runtime
        .shutdown("shutdown-dummy")
        .expect("shutdown ACP runtime");

    drop(database);
    cleanup(&root);
}

#[test]
fn host_forcibly_terminates_a_non_cooperative_timed_out_prompt() {
    let runtime = hanging_runtime(Duration::from_millis(150));
    let (sender, receiver) = mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        let started_at = Instant::now();
        let mut runtime = runtime;
        let result = runtime.invoke_cancellable(hanging_invocation(), Arc::new(|_| {}), || false);
        let _ = sender.send((result, started_at.elapsed()));
    });

    let (result, elapsed) = receiver
        .recv_timeout(Duration::from_secs(3))
        .expect("timed out prompt must return");
    assert!(matches!(
        result,
        Err(SupervisorError::Timeout("ACP prompt"))
    ));
    assert!(elapsed < Duration::from_secs(2));
    thread.join().expect("runtime thread");
}

#[test]
fn host_forcibly_terminates_a_non_cooperative_cancelled_prompt() {
    let root = temporary_root("process-tree-cancellation");
    cleanup(&root);
    std::fs::create_dir_all(&root).expect("create process-tree test root");
    let started = root.join("started");
    let descendant = root.join("descendant");
    let mut runtime = hanging_runtime(TEST_TIMEOUT);
    let started_at = Instant::now();
    let result = runtime.invoke_cancellable(
        ExtensionRuntimeInvocation::new(
            "invoke-process-tree",
            1,
            "prompt",
            "prompt",
            format!(
                "@com.example.acp-dummy spawn-child|{}|{}",
                started.display(),
                descendant.display()
            ),
        ),
        Arc::new(|_| {}),
        || started.exists(),
    );
    let elapsed = started_at.elapsed();
    assert!(matches!(
        result,
        Err(SupervisorError::Cancelled("ACP prompt"))
    ));
    assert!(elapsed < Duration::from_secs(2));
    assert!(started.exists());

    assert!(
        runtime
            .recover_if_exited("recover-after-cancel")
            .expect("recover runtime")
    );
    let streamed = Arc::new(Mutex::new(String::new()));
    let published = Arc::clone(&streamed);
    runtime
        .invoke_cancellable(
            ExtensionRuntimeInvocation::new(
                "invoke-after-cancel",
                2,
                "prompt",
                "prompt",
                "@com.example.acp-dummy hello",
            ),
            Arc::new(move |chunk| {
                published
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .push_str(&chunk);
            }),
            || false,
        )
        .expect("recovered runtime should accept another prompt");
    assert_eq!(
        streamed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_str(),
        "Hello World"
    );
    runtime
        .shutdown("shutdown-after-cancel")
        .expect("shutdown runtime");
    std::thread::sleep(Duration::from_millis(700));
    assert!(
        !descendant.exists(),
        "descendant process must be terminated"
    );
    cleanup(&root);
}

#[test]
fn host_contains_a_descendant_spawned_at_extension_startup() {
    let root = temporary_root("startup-process-tree");
    cleanup(&root);
    std::fs::create_dir_all(&root).expect("create process-tree test root");
    let started = root.join("started");
    let descendant = root.join("descendant");
    let argument = format!(
        "--spawn-child-at-start={}|{}",
        started.display(),
        descendant.display()
    );
    let mut runtime = hanging_runtime_with_arguments(TEST_TIMEOUT, [argument.into()]);
    assert!(started.exists(), "startup descendant should be spawned");

    runtime
        .terminate()
        .expect("terminate extension process tree");
    std::thread::sleep(Duration::from_millis(700));
    assert!(
        !descendant.exists(),
        "startup descendant process must be terminated"
    );
    cleanup(&root);
}

#[test]
fn coordinator_cancels_an_acp_invocation_and_keeps_the_extension_usable() {
    let root = temporary_root("coordinator-cancellation");
    cleanup(&root);
    std::fs::create_dir_all(&root).expect("create cancellation test root");
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "com.example.acp-dummy",
            hanging_runtime(TEST_TIMEOUT),
            owner.handle(),
        )
        .expect("register ACP extension");
    for cycle in 0_u64..4 {
        let invocation_id = coordinator
            .invoke(
                "com.example.acp-dummy",
                cycle * 2 + 1,
                "prompt",
                "prompt",
                "@com.example.acp-dummy hang",
            )
            .expect("enqueue hanging prompt");
        coordinator
            .cancel_invocation("com.example.acp-dummy", invocation_id)
            .expect("cancel hanging prompt");

        let completed = root.join(format!("completed-{cycle}"));
        coordinator
            .invoke(
                "com.example.acp-dummy",
                cycle * 2 + 2,
                "prompt",
                "prompt",
                format!("@com.example.acp-dummy mark|{}", completed.display()),
            )
            .expect("enqueue prompt after cancellation");
        let recovery_deadline = Instant::now() + Duration::from_secs(3);
        while !completed.exists() {
            assert!(
                Instant::now() < recovery_deadline,
                "extension should complete a prompt after cancellation"
            );
            std::thread::yield_now();
        }
        assert!(coordinator.first_error().is_none());
    }
    drop(coordinator);
    owner.shutdown();
    cleanup(&root);
}

#[test]
fn coordinator_shutdown_does_not_restart_a_cancelled_extension() {
    let root = temporary_root("coordinator-shutdown");
    cleanup(&root);
    std::fs::create_dir_all(&root).expect("create shutdown test root");
    let starts = root.join("starts");
    let invocation_started = root.join("invocation-started");
    let argument = format!("--append-start-marker={}", starts.display());
    let owner = SearchOwner::spawn(UsageMap::new()).expect("search owner");
    let mut coordinator = ExtensionSearchCoordinator::default();
    coordinator
        .register(
            "com.example.acp-dummy",
            hanging_runtime_with_arguments(TEST_TIMEOUT, [argument.into()]),
            owner.handle(),
        )
        .expect("register ACP extension");
    coordinator
        .invoke(
            "com.example.acp-dummy",
            1,
            "prompt",
            "prompt",
            format!(
                "@com.example.acp-dummy hang-after|{}",
                invocation_started.display()
            ),
        )
        .expect("enqueue hanging prompt");
    let invocation_deadline = Instant::now() + Duration::from_secs(3);
    while !invocation_started.exists() {
        assert!(
            Instant::now() < invocation_deadline,
            "hanging prompt should start"
        );
        std::thread::yield_now();
    }

    coordinator.shutdown();
    let starts = std::fs::read_to_string(&starts).expect("read startup markers");
    assert_eq!(starts.lines().count(), 1);
    owner.shutdown();
    cleanup(&root);
}

fn run_with_timeout(operation: impl Future<Output = agent_client_protocol::Result<()>>) {
    async_io::block_on(future::race(operation, async {
        async_io::Timer::after(TEST_TIMEOUT).await;
        Err(Error::internal_error().data("ACP test timed out"))
    }))
    .expect("ACP v1 round trip should succeed");
}

fn create_package(path: &Path, executable: &Path) {
    std::fs::create_dir_all(path.parent().expect("package parent")).expect("create parent");
    let file = std::fs::File::create(path).expect("create package");
    let mut archive = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();
    let target = if cfg!(windows) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-apple-darwin"
    };
    let entrypoint = format!("bin/{target}/acp-dummy{}", std::env::consts::EXE_SUFFIX);
    let manifest = serde_json::json!({
        "format": "nanika-extension",
        "manifestVersion": 1,
        "id": "com.example.acp-dummy",
        "version": "0.1.0",
        "hostApi": "^0.1",
        "targets": {
            (target): {
                "entrypoint": entrypoint.clone()
            }
        },
        "runtime": {
            "protocol": "acp",
            "protocolVersion": 1
        }
    });
    archive
        .start_file("manifest.jsonc", options)
        .expect("manifest entry");
    archive
        .write_all(manifest.to_string().as_bytes())
        .expect("manifest contents");
    archive
        .start_file(&entrypoint, options)
        .expect("binary entry");
    let mut binary = std::fs::File::open(executable).expect("open dummy executable");
    std::io::copy(&mut binary, &mut archive).expect("binary contents");
    archive.finish().expect("finish package");
}

fn dummy_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nanika-extension-acp-dummy"))
}

fn hanging_runtime(action_timeout: Duration) -> ExtensionRuntime {
    hanging_runtime_with_arguments(action_timeout, std::iter::empty())
}

fn hanging_runtime_with_arguments(
    action_timeout: Duration,
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> ExtensionRuntime {
    let limits = ExtensionLimits {
        handshake_timeout: TEST_TIMEOUT,
        action_timeout,
        shutdown_timeout: Duration::from_millis(500),
        ..ExtensionLimits::default()
    };
    let mut runtime = ExtensionRuntime::spawn_with(
        "com.example.acp-dummy",
        ExtensionProtocol::Acp {
            protocol_version: 1,
        },
        dummy_executable(),
        arguments,
        limits,
    )
    .expect("spawn ACP runtime");
    runtime
        .initialize("initialize-hanging-dummy")
        .expect("initialize ACP runtime");
    runtime
}

fn hanging_invocation() -> ExtensionRuntimeInvocation {
    ExtensionRuntimeInvocation::new(
        "invoke-hanging-dummy",
        1,
        "prompt",
        "prompt",
        "@com.example.acp-dummy hang",
    )
}

fn temporary_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "nanika-acp-dummy-{label}-{}-{unique}",
        std::process::id()
    ))
}

fn cleanup(path: &Path) {
    if path.exists() {
        std::fs::remove_dir_all(path).expect("cleanup");
    }
}
