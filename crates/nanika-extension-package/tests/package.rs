use std::io::Write;

use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{
    ExtensionProtocol, install_package, remove_extension, resolve_active_extensions,
    set_extension_enabled, update_package,
};
use nanika_storage::{HostDatabase, NanikaPaths};
use zip::write::SimpleFileOptions;

#[test]
fn package_install_enablement_resolution_and_removal_round_trip() {
    let root = temporary_root("round-trip");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("example.nanika");
    create_package(&package, false);

    let installed = install_package(&package, &paths, &store).expect("install package");
    assert_eq!(installed.extension_id, "com.example.extension");
    assert_eq!(
        installed.protocol,
        ExtensionProtocol::Nanika {
            protocol_version: 1
        }
    );
    assert!(installed.program.is_file());

    let database = HostDatabase::open(paths.host_database()).expect("database");
    let records = database.load_extensions().expect("load extensions");
    let registry = ExtensionRegistryConfig::load(&store).expect("registry");
    let (active, errors) = resolve_active_extensions(&paths, &records, &registry);
    assert!(errors.is_empty());
    assert_eq!(active.len(), 1);

    set_extension_enabled("com.example.extension", false, &paths, &store)
        .expect("disable extension");
    let registry = ExtensionRegistryConfig::load(&store).expect("registry");
    let (active, errors) = resolve_active_extensions(&paths, &records, &registry);
    assert!(errors.is_empty());
    assert!(active.is_empty());

    drop(database);
    remove_extension("com.example.extension", &paths, &store).expect("remove extension");
    assert!(
        HostDatabase::open(paths.host_database())
            .expect("database")
            .extension("com.example.extension")
            .expect("extension lookup")
            .is_none()
    );
    cleanup(&root);
}

#[test]
fn manifest_version_two_preserves_acp_protocol_version() {
    let root = temporary_root("acp-runtime");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("acp.nanika");
    create_package_definition_with_runtime(
        &package,
        false,
        &[],
        "1.2.3",
        false,
        false,
        2,
        Some(serde_json::json!({
            "protocol": "acp",
            "protocolVersion": 1
        })),
    );

    let installed = install_package(&package, &paths, &store).expect("install ACP package");

    assert_eq!(
        installed.protocol,
        ExtensionProtocol::Acp {
            protocol_version: 1
        }
    );
    cleanup(&root);
}

#[test]
fn manifest_version_two_preserves_nanika_protocol_version() {
    let root = temporary_root("nanika-runtime-v2");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("nanika.nanika");
    create_package_definition_with_runtime(
        &package,
        false,
        &[],
        "1.2.3",
        false,
        false,
        2,
        Some(serde_json::json!({
            "protocol": "nanika",
            "protocolVersion": 1
        })),
    );

    let installed = install_package(&package, &paths, &store).expect("install Nanika package");

    assert_eq!(
        installed.protocol,
        ExtensionProtocol::Nanika {
            protocol_version: 1
        }
    );
    cleanup(&root);
}

#[test]
fn manifest_version_two_requires_runtime_protocol() {
    let root = temporary_root("missing-runtime-v2");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("missing-runtime.nanika");
    create_package_definition_with_runtime(&package, false, &[], "1.2.3", false, false, 2, None);

    let error = install_package(&package, &paths, &store)
        .expect_err("manifest version 2 without a runtime must fail");

    assert!(error.to_string().contains("requires a runtime protocol"));
    cleanup(&root);
}

#[test]
fn manifest_version_two_rejects_unsupported_acp_protocol_version() {
    let root = temporary_root("unsupported-acp-runtime");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("acp.nanika");
    create_package_definition_with_runtime(
        &package,
        false,
        &[],
        "1.2.3",
        false,
        false,
        2,
        Some(serde_json::json!({
            "protocol": "acp",
            "protocolVersion": 2
        })),
    );

    let error = install_package(&package, &paths, &store)
        .expect_err("unsupported ACP protocol version must fail");

    assert!(
        error
            .to_string()
            .contains("unsupported ACP protocol version")
    );
    cleanup(&root);
}

#[test]
fn manifest_version_one_rejects_runtime_protocol_declarations() {
    let root = temporary_root("legacy-runtime");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("legacy.nanika");
    create_package_definition_with_runtime(
        &package,
        false,
        &[],
        "1.2.3",
        false,
        false,
        1,
        Some(serde_json::json!({
            "protocol": "acp",
            "protocolVersion": 1
        })),
    );

    let error = install_package(&package, &paths, &store)
        .expect_err("legacy manifest runtime declaration must fail");

    assert!(error.to_string().contains("version 1 cannot declare"));
    cleanup(&root);
}

#[test]
fn package_rejects_path_traversal() {
    let root = temporary_root("traversal");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("traversal.nanika");
    create_package(&package, true);

    assert!(install_package(&package, &paths, &store).is_err());
    assert!(!root.join("escape.txt").exists());
    cleanup(&root);
}

#[test]
fn enablement_rejects_unknown_extensions_without_changing_config() {
    let root = temporary_root("unknown-enablement");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");

    let error = set_extension_enabled("com.example.missing", false, &paths, &store)
        .expect_err("unknown extension should fail");

    assert!(error.to_string().contains("not installed"));
    let registry = ExtensionRegistryConfig::load(&store).expect("registry");
    assert!(registry.extensions.is_empty());
    cleanup(&root);
}

#[test]
fn same_version_update_repairs_content_and_preserves_disablement() {
    let root = temporary_root("repair-disabled");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("example.nanika");
    create_package(&package, false);
    let installed = install_package(&package, &paths, &store).expect("install package");
    set_extension_enabled("com.example.extension", false, &paths, &store)
        .expect("disable extension");
    std::fs::write(&installed.program, "corrupt").expect("corrupt installed executable");

    let repaired = install_package(&package, &paths, &store).expect("repair package");

    assert_eq!(
        std::fs::read(&repaired.program).expect("repaired executable"),
        b"fixture"
    );
    let database = HostDatabase::open(paths.host_database()).expect("database");
    assert_eq!(
        database
            .extension("com.example.extension")
            .expect("extension lookup")
            .expect("extension record")
            .state,
        "disabled"
    );
    assert!(
        !ExtensionRegistryConfig::load(&store)
            .expect("registry")
            .is_enabled("com.example.extension", true)
    );
    drop(database);
    cleanup(&root);
}

#[test]
fn removal_preflights_registry_before_mutating_artifacts_or_storage() {
    let root = temporary_root("remove-preflight");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("example.nanika");
    create_package(&package, false);
    let installed = install_package(&package, &paths, &store).expect("install package");
    std::fs::write(store.extensions_file(), "{ malformed").expect("malformed registry fixture");

    assert!(remove_extension("com.example.extension", &paths, &store).is_err());
    assert!(installed.program.is_file());
    assert!(
        HostDatabase::open(paths.host_database())
            .expect("database")
            .extension("com.example.extension")
            .expect("extension lookup")
            .is_some()
    );
    cleanup(&root);
}

#[test]
fn unresolved_manifest_dependencies_are_rejected_explicitly() {
    let root = temporary_root("dependencies");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("dependency.nanika");
    create_package_with_dependencies(&package, false, &["com.example.required"]);

    let error = install_package(&package, &paths, &store)
        .expect_err("unsupported dependencies should fail");
    assert!(error.to_string().contains("dependency resolver"));
    cleanup(&root);
}

#[test]
fn install_and_update_enforce_existence_and_version_direction() {
    let root = temporary_root("operation-semantics");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("example.nanika");
    create_package_definition(&package, false, &[], "1.2.3", false, false);

    assert!(update_package(&package, &paths, &store).is_err());
    install_package(&package, &paths, &store).expect("initial install");

    create_package_definition(&package, false, &[], "1.0.0", false, false);
    let downgrade = update_package(&package, &paths, &store).expect_err("downgrade must fail");
    assert!(downgrade.to_string().contains("cannot downgrade"));

    create_package_definition(&package, false, &[], "2.0.0", false, false);
    let reinstall = install_package(&package, &paths, &store)
        .expect_err("install must not replace a different version");
    assert!(reinstall.to_string().contains("use update"));
    let updated = update_package(&package, &paths, &store).expect("newer update");
    assert!(updated.program.to_string_lossy().contains("2.0.0"));
    cleanup(&root);
}

#[test]
fn manifest_rejects_unknown_fields() {
    let root = temporary_root("unknown-manifest-field");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("unknown.nanika");
    create_package_definition(&package, false, &[], "1.2.3", true, false);

    let error = install_package(&package, &paths, &store).expect_err("unknown field must fail");
    assert!(error.to_string().contains("unknown field"));
    cleanup(&root);
}

#[cfg(windows)]
#[test]
fn package_rejects_unicode_filesystem_collisions() {
    let root = temporary_root("unicode-collision");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("collision.nanika");
    create_package_definition(&package, false, &[], "1.2.3", false, true);

    let error = install_package(&package, &paths, &store).expect_err("collision must fail");
    assert!(error.to_string().contains("colliding paths"));
    cleanup(&root);
}

#[test]
fn interrupted_same_version_replacement_recovers_before_resolution() {
    let root = temporary_root("replacement-recovery");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("example.nanika");
    create_package(&package, false);
    install_package(&package, &paths, &store).expect("install package");
    let database = HostDatabase::open(paths.host_database()).expect("database");
    let installed = database
        .extension("com.example.extension")
        .expect("extension lookup")
        .expect("extension row");
    let version_root = installed.install_path.as_ref().expect("install path");
    let extension_root = version_root.parent().expect("extension root");
    let backup_name = ".replaced-1.2.3-interrupted";
    std::fs::rename(version_root, extension_root.join(backup_name)).expect("interrupt replacement");
    std::fs::write(
        paths
            .app_data_root()
            .join("extensions")
            .join(".package-transaction.json"),
        serde_json::to_vec(&serde_json::json!({
            "operation": "replace",
            "extensionId": "com.example.extension",
            "version": "1.2.3",
            "backupName": backup_name
        }))
        .expect("journal"),
    )
    .expect("write journal");
    let records = database.load_extensions().expect("extension records");
    let registry = ExtensionRegistryConfig::load(&store).expect("registry");

    let (active, errors) = resolve_active_extensions(&paths, &records, &registry);

    assert!(errors.is_empty());
    assert_eq!(active.len(), 1);
    assert!(version_root.is_dir());
    assert!(
        !paths
            .app_data_root()
            .join("extensions")
            .join(".package-transaction.json")
            .exists()
    );
    drop(database);
    cleanup(&root);
}

#[test]
fn interrupted_removal_recovers_before_resolution() {
    let root = temporary_root("removal-recovery");
    cleanup(&root);
    let paths = NanikaPaths::from_roots(
        root.join("data"),
        root.join("cache"),
        root.join("config-default"),
    );
    let store = ConfigStore::open(paths.app_data_root(), paths.config_root()).expect("store");
    let package = root.join("example.nanika");
    create_package(&package, false);
    install_package(&package, &paths, &store).expect("install package");
    let database = HostDatabase::open(paths.host_database()).expect("database");
    let installed = database
        .extension("com.example.extension")
        .expect("extension lookup")
        .expect("extension row");
    let version_root = installed.install_path.as_ref().expect("install path");
    let extension_root = version_root.parent().expect("extension root");
    let extensions_root = extension_root.parent().expect("extensions root");
    let backup_name = ".removed-com.example.extension-interrupted";
    std::fs::rename(extension_root, extensions_root.join(backup_name)).expect("interrupt removal");
    std::fs::write(
        extensions_root.join(".package-transaction.json"),
        serde_json::to_vec(&serde_json::json!({
            "operation": "remove",
            "extensionId": "com.example.extension",
            "version": null,
            "backupName": backup_name
        }))
        .expect("journal"),
    )
    .expect("write journal");
    let records = database.load_extensions().expect("extension records");
    let registry = ExtensionRegistryConfig::load(&store).expect("registry");

    let (active, errors) = resolve_active_extensions(&paths, &records, &registry);

    assert!(errors.is_empty());
    assert_eq!(active.len(), 1);
    assert!(extension_root.is_dir());
    assert!(!extensions_root.join(".package-transaction.json").exists());
    drop(database);
    cleanup(&root);
}

fn create_package(path: &std::path::Path, traversal: bool) {
    create_package_with_dependencies(path, traversal, &[]);
}

fn create_package_with_dependencies(
    path: &std::path::Path,
    traversal: bool,
    dependencies: &[&str],
) {
    create_package_definition(path, traversal, dependencies, "1.2.3", false, false);
}

fn create_package_definition(
    path: &std::path::Path,
    traversal: bool,
    dependencies: &[&str],
    version: &str,
    unknown_field: bool,
    unicode_collision: bool,
) {
    create_package_definition_with_runtime(
        path,
        traversal,
        dependencies,
        version,
        unknown_field,
        unicode_collision,
        1,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn create_package_definition_with_runtime(
    path: &std::path::Path,
    traversal: bool,
    dependencies: &[&str],
    version: &str,
    unknown_field: bool,
    unicode_collision: bool,
    manifest_version: u32,
    runtime: Option<serde_json::Value>,
) {
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
    let entrypoint = format!("bin/{target}/example{}", std::env::consts::EXE_SUFFIX);
    let mut manifest = serde_json::json!({
        "format": "nanika-extension",
        "manifestVersion": manifest_version,
        "id": "com.example.extension",
        "version": version,
        "hostApi": "^0.1",
        "targets": {
            (target): { "entrypoint": entrypoint.clone() }
        },
        "permissions": ["process.launch"],
        "dependencies": dependencies
    });
    if let Some(runtime) = runtime {
        manifest["runtime"] = runtime;
    }
    if unknown_field {
        manifest["dependecies"] = serde_json::json!(["com.example.required"]);
    }
    archive
        .start_file("manifest.jsonc", options)
        .expect("start manifest");
    archive
        .write_all(
            serde_json::to_string_pretty(&manifest)
                .expect("manifest")
                .as_bytes(),
        )
        .expect("write manifest");
    archive
        .start_file(&entrypoint, options)
        .expect("start entrypoint");
    archive.write_all(b"fixture").expect("write entrypoint");
    if unicode_collision {
        archive
            .start_file("resources/Ä.txt", options)
            .expect("start first collision entry");
        archive.write_all(b"first").expect("write first collision");
        archive
            .start_file("resources/ä.txt", options)
            .expect("start second collision entry");
        archive
            .write_all(b"second")
            .expect("write second collision");
    }
    if traversal {
        archive
            .start_file("../escape.txt", options)
            .expect("start traversal");
        archive.write_all(b"escape").expect("write traversal");
    }
    archive.finish().expect("finish package");
}

fn temporary_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nanika-extension-package-{name}-{}",
        std::process::id()
    ))
}

fn cleanup(root: &std::path::Path) {
    let _ = std::fs::remove_dir_all(root);
}
