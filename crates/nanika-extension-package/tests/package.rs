use std::io::Write;

use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_extension_package::{
    install_package, remove_extension, resolve_active_extensions, set_extension_enabled,
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

fn create_package(path: &std::path::Path, traversal: bool) {
    create_package_with_dependencies(path, traversal, &[]);
}

fn create_package_with_dependencies(
    path: &std::path::Path,
    traversal: bool,
    dependencies: &[&str],
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
    let manifest = serde_json::json!({
        "format": "nanika-extension",
        "manifestVersion": 1,
        "id": "com.example.extension",
        "version": "1.2.3",
        "hostApi": "^0.1",
        "targets": {
            (target): { "entrypoint": entrypoint.clone() }
        },
        "permissions": ["process.launch"],
        "dependencies": dependencies
    });
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
