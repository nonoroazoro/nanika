use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use jsonc_parser::{ParseOptions, parse_to_serde_value};
use nanika_config::{ConfigStore, ExtensionRegistryConfig};
use nanika_storage::{ExtensionKind, HostDatabase, NanikaPaths, StoredExtension};
use semver::{Version, VersionReq};
use uuid::Uuid;
use zip::{CompressionMethod, ZipArchive};

use crate::{
    ActiveExtension, ExtensionManifest, ExtensionPackageError, ExtensionResolutionError,
    ExtensionTarget, PackageOperation, PackageTransaction, StagedPackage, StagingDirectory,
};

const MANIFEST_FORMAT: &str = "nanika-extension";
const MANIFEST_VERSION: u32 = 1;
const MAX_PACKAGE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_ENTRIES: usize = 4_096;
const MAX_COMPRESSION_RATIO: u64 = 100;

/// Validate, stage, and activate one local `.nanika` package.
pub fn install_package(
    package_path: &Path,
    paths: &NanikaPaths,
    store: &ConfigStore,
) -> Result<ActiveExtension, ExtensionPackageError> {
    apply_package(package_path, paths, store, PackageOperation::Install)
}

/// Validate and activate a newer local `.nanika` package for an installed extension.
pub fn update_package(
    package_path: &Path,
    paths: &NanikaPaths,
    store: &ConfigStore,
) -> Result<ActiveExtension, ExtensionPackageError> {
    apply_package(package_path, paths, store, PackageOperation::Update)
}

fn apply_package(
    package_path: &Path,
    paths: &NanikaPaths,
    store: &ConfigStore,
    operation: PackageOperation,
) -> Result<ActiveExtension, ExtensionPackageError> {
    validate_package_path(package_path)?;
    let extension_root = paths.app_data_root().join("extensions");
    fs::create_dir_all(&extension_root)?;
    recover_package_artifacts(paths)?;
    let staged_package = StagedPackage::create(
        package_path,
        extension_root.join(format!(".package-{}.partial", Uuid::new_v4())),
        MAX_PACKAGE_BYTES,
    )?;
    let digest = staged_package.digest().to_owned();
    let stage_path = extension_root.join(format!(".staging-{}", Uuid::new_v4()));
    let mut stage = StagingDirectory::create(stage_path)?;
    extract_package(staged_package.path(), stage.path())?;

    let manifest = load_manifest(stage.path().join("manifest.jsonc"))?;
    validate_manifest(&manifest)?;
    let target = current_target();
    let entrypoint = target_entrypoint(&manifest, target)?;
    let staged_program = stage.path().join(&entrypoint);
    if !staged_program.is_file() {
        return Err(ExtensionPackageError::Manifest(format!(
            "target entrypoint is missing: {}",
            entrypoint.display()
        )));
    }
    make_executable(&staged_program)?;
    fs::write(stage.path().join(".package.sha256"), format!("{digest}\n"))?;

    let database = HostDatabase::open(paths.host_database())?;
    let previous = database.extension(&manifest.id)?;
    if previous
        .as_ref()
        .is_some_and(|extension| extension.kind == ExtensionKind::BuiltIn)
    {
        return Err(ExtensionPackageError::Manifest(
            "an external package cannot replace a built-in extension".to_owned(),
        ));
    }
    validate_package_operation(operation, &manifest.version, previous.as_ref())?;
    let mut registry =
        ExtensionRegistryConfig::load(store).map_err(ExtensionPackageError::Config)?;
    let registry_existed = store.extensions_file().is_file();
    let original_registry = registry.clone();
    let enabled = previous
        .as_ref()
        .is_none_or(|extension| registry.is_enabled(&manifest.id, extension.state == "enabled"));

    let extension_id_root = prepare_extension_root(&extension_root, &manifest.id)?;
    let version_root = extension_id_root.join(&manifest.version);
    let mut replacement_transaction = None;
    let replaced_root = if version_root.exists() {
        validate_version_directory(&extension_root, &version_root)?;
        let existing_digest = fs::read_to_string(version_root.join(".package.sha256"))?;
        if existing_digest.trim() != digest {
            return Err(ExtensionPackageError::Manifest(format!(
                "extension version {} is immutable and has a different digest",
                manifest.version
            )));
        }
        let replaced =
            extension_id_root.join(format!(".replaced-{}-{}", manifest.version, Uuid::new_v4()));
        let transaction = PackageTransaction::replacement(
            &manifest.id,
            &manifest.version,
            replaced
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    ExtensionPackageError::Manifest(
                        "replacement backup name is not valid UTF-8".to_owned(),
                    )
                })?,
        );
        transaction.save(&extension_root)?;
        if let Err(error) = fs::rename(&version_root, &replaced) {
            PackageTransaction::clear(&extension_root)?;
            return Err(error.into());
        }
        replacement_transaction = Some(transaction);
        Some(replaced)
    } else {
        None
    };
    if let Err(error) = fs::rename(stage.path(), &version_root) {
        if let Some(replaced) = &replaced_root {
            fs::rename(replaced, &version_root)?;
            PackageTransaction::clear(&extension_root)?;
        }
        return Err(error.into());
    }
    stage.commit();

    registry.set_enabled(&manifest.id, enabled);
    if let Err(error) = registry.save(store) {
        rollback_version(&version_root, replaced_root.as_deref())?;
        if replacement_transaction.is_some() {
            PackageTransaction::clear(&extension_root)?;
        }
        return Err(ExtensionPackageError::Config(error));
    }
    if let Err(error) = database.install_external_extension(
        &manifest.id,
        &manifest.version,
        &version_root,
        &digest,
        enabled,
        unix_timestamp(),
    ) {
        let config_rollback = restore_registry(store, &original_registry, registry_existed);
        let artifact_rollback = rollback_version(&version_root, replaced_root.as_deref());
        if let Err(rollback) = config_rollback {
            return Err(ExtensionPackageError::Config(format!(
                "extension storage failed: {error}; configuration rollback failed: {rollback}"
            )));
        }
        artifact_rollback?;
        if replacement_transaction.is_some() {
            PackageTransaction::clear(&extension_root)?;
        }
        return Err(error.into());
    }
    if let Some(replaced) = replaced_root {
        let _ = fs::remove_dir_all(replaced);
    }
    if replacement_transaction.is_some() {
        PackageTransaction::clear(&extension_root)?;
    }

    let protocol = manifest.validated_protocol();
    Ok(ActiveExtension {
        extension_id: manifest.id,
        program: version_root.join(entrypoint),
        protocol,
        permissions: manifest.permissions,
    })
}

/// Update synchronized enablement and mirror external state in the host database.
pub fn set_extension_enabled(
    extension_id: &str,
    enabled: bool,
    paths: &NanikaPaths,
    store: &ConfigStore,
) -> Result<(), ExtensionPackageError> {
    if !nanika_storage::is_valid_extension_id(extension_id) {
        return Err(ExtensionPackageError::Manifest(
            "invalid extension id".to_owned(),
        ));
    }
    recover_package_artifacts(paths)?;
    let database = HostDatabase::open(paths.host_database())?;
    let installed = database
        .extension(extension_id)?
        .ok_or_else(|| ExtensionPackageError::Manifest("extension is not installed".to_owned()))?;
    if installed.kind != ExtensionKind::External {
        return Err(ExtensionPackageError::Manifest(
            "only external extensions can be changed by the package manager".to_owned(),
        ));
    }
    let mut registry =
        ExtensionRegistryConfig::load(store).map_err(ExtensionPackageError::Config)?;
    let registry_existed = store.extensions_file().is_file();
    let original_registry = registry.clone();
    registry.set_enabled(extension_id, enabled);
    registry
        .save(store)
        .map_err(ExtensionPackageError::Config)?;
    match database.set_external_extension_enabled(extension_id, enabled, unix_timestamp()) {
        Ok(true) => {}
        Ok(false) => {
            restore_registry(store, &original_registry, registry_existed)
                .map_err(ExtensionPackageError::Config)?;
            return Err(ExtensionPackageError::Manifest(
                "external extension state changed unexpectedly".to_owned(),
            ));
        }
        Err(error) => {
            restore_registry(store, &original_registry, registry_existed).map_err(|rollback| {
                ExtensionPackageError::Config(format!(
                    "extension storage failed: {error}; configuration rollback failed: {rollback}"
                ))
            })?;
            return Err(error.into());
        }
    }
    Ok(())
}

/// Remove external executable versions while preserving extension settings and data.
pub fn remove_extension(
    extension_id: &str,
    paths: &NanikaPaths,
    store: &ConfigStore,
) -> Result<(), ExtensionPackageError> {
    if !nanika_storage::is_valid_extension_id(extension_id) {
        return Err(ExtensionPackageError::Manifest(
            "invalid extension id".to_owned(),
        ));
    }
    recover_package_artifacts(paths)?;
    let database = HostDatabase::open(paths.host_database())?;
    let installed = database
        .extension(extension_id)?
        .ok_or_else(|| ExtensionPackageError::Manifest("extension is not installed".to_owned()))?;
    if installed.kind != ExtensionKind::External {
        return Err(ExtensionPackageError::Manifest(
            "built-in extensions cannot be removed".to_owned(),
        ));
    }
    let mut registry =
        ExtensionRegistryConfig::load(store).map_err(ExtensionPackageError::Config)?;
    let registry_existed = store.extensions_file().is_file();
    let original_registry = registry.clone();
    let extensions_root = paths.app_data_root().join("extensions");
    let extension_root = extensions_root.join(extension_id);
    validate_managed_path(&extensions_root, &extension_root)?;
    let removed_root =
        extensions_root.join(format!(".removed-{}-{}", extension_id, Uuid::new_v4()));
    let mut removal_transaction = None;
    if extension_root.exists() {
        let transaction = PackageTransaction::removal(
            extension_id,
            removed_root
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    ExtensionPackageError::Manifest(
                        "removal backup name is not valid UTF-8".to_owned(),
                    )
                })?,
        );
        transaction.save(&extensions_root)?;
        if let Err(error) = fs::rename(&extension_root, &removed_root) {
            PackageTransaction::clear(&extensions_root)?;
            return Err(error.into());
        }
        removal_transaction = Some(transaction);
    }
    registry.remove(extension_id);
    if let Err(error) = registry.save(store) {
        if removed_root.exists() {
            fs::rename(&removed_root, &extension_root).map_err(|rollback| {
                ExtensionPackageError::Config(format!(
                    "{error}; artifact rollback failed: {rollback}"
                ))
            })?;
        }
        if removal_transaction.is_some() {
            PackageTransaction::clear(&extensions_root)?;
        }
        return Err(ExtensionPackageError::Config(error));
    }
    let database_result = database.remove_external_extension(extension_id);
    if !matches!(database_result, Ok(true)) {
        let config_rollback = restore_registry(store, &original_registry, registry_existed);
        let artifact_rollback = if removed_root.exists() {
            fs::rename(&removed_root, &extension_root)
        } else {
            Ok(())
        };
        if let Err(rollback) = config_rollback {
            return Err(ExtensionPackageError::Config(format!(
                "extension removal failed; configuration rollback failed: {rollback}"
            )));
        }
        if let Err(rollback) = artifact_rollback {
            return Err(ExtensionPackageError::Config(format!(
                "extension removal failed; artifact rollback failed: {rollback}"
            )));
        }
        if removal_transaction.is_some() {
            PackageTransaction::clear(&extensions_root)?;
        }
        return match database_result {
            Ok(false) => Err(ExtensionPackageError::Manifest(
                "extension is not removable".to_owned(),
            )),
            Err(error) => Err(error.into()),
            Ok(true) => unreachable!("successful removal returns before rollback"),
        };
    }
    if removed_root.exists() {
        let _ = fs::remove_dir_all(removed_root);
    }
    if removal_transaction.is_some() {
        PackageTransaction::clear(&extensions_root)?;
    }
    Ok(())
}

/// Resolve installed external processes without allowing one broken package to block startup.
pub fn resolve_active_extensions(
    paths: &NanikaPaths,
    installed: &[StoredExtension],
    registry: &ExtensionRegistryConfig,
) -> (Vec<ActiveExtension>, Vec<ExtensionResolutionError>) {
    let mut active = Vec::new();
    let mut errors = Vec::new();
    if let Err(error) = recover_package_artifacts(paths) {
        errors.push(ExtensionResolutionError::new(
            "package-recovery",
            format!("package recovery failed: {error}"),
        ));
    }
    for extension in installed
        .iter()
        .filter(|extension| extension.kind == ExtensionKind::External)
    {
        let default_enabled = extension.state == "enabled";
        if !registry.is_enabled(&extension.extension_id, default_enabled) {
            continue;
        }
        match resolve_active_extension(paths, extension) {
            Ok(extension) => active.push(extension),
            Err(error) => errors.push(ExtensionResolutionError::new(
                &extension.extension_id,
                error.to_string(),
            )),
        }
    }
    (active, errors)
}

fn resolve_active_extension(
    paths: &NanikaPaths,
    installed: &StoredExtension,
) -> Result<ActiveExtension, ExtensionPackageError> {
    let install_path = installed.install_path.as_deref().ok_or_else(|| {
        ExtensionPackageError::Manifest("active install path is missing".to_owned())
    })?;
    let extensions_root = paths.app_data_root().join("extensions");
    validate_managed_path(&extensions_root, install_path)?;
    let canonical_root = fs::canonicalize(&extensions_root)?;
    let canonical_install = fs::canonicalize(install_path)?;
    validate_managed_path(&canonical_root, &canonical_install)?;
    let manifest = load_manifest(canonical_install.join("manifest.jsonc"))?;
    validate_manifest(&manifest)?;
    if manifest.id != installed.extension_id
        || installed.active_version.as_deref() != Some(manifest.version.as_str())
    {
        return Err(ExtensionPackageError::Manifest(
            "installed manifest identity does not match host state".to_owned(),
        ));
    }
    let entrypoint = target_entrypoint(&manifest, current_target())?;
    let program = canonical_install.join(entrypoint);
    validate_managed_path(&canonical_install, &program)?;
    if !program.is_file() {
        return Err(ExtensionPackageError::Manifest(
            "active target entrypoint is missing".to_owned(),
        ));
    }
    let program = fs::canonicalize(program)?;
    validate_managed_path(&canonical_install, &program)?;
    let protocol = manifest.validated_protocol();
    Ok(ActiveExtension {
        extension_id: manifest.id,
        program,
        protocol,
        permissions: manifest.permissions,
    })
}

fn validate_package_operation(
    operation: PackageOperation,
    package_version: &str,
    previous: Option<&StoredExtension>,
) -> Result<(), ExtensionPackageError> {
    let Some(previous) = previous else {
        return if operation == PackageOperation::Install {
            Ok(())
        } else {
            Err(ExtensionPackageError::Manifest(
                "update requires an installed external extension".to_owned(),
            ))
        };
    };
    let current = previous
        .active_version
        .as_deref()
        .or(previous.installed_version.as_deref())
        .ok_or_else(|| {
            ExtensionPackageError::Manifest("installed extension has no current version".to_owned())
        })?;
    let current = Version::parse(current)
        .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))?;
    let package = Version::parse(package_version)
        .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))?;
    match operation {
        PackageOperation::Install if package != current => Err(ExtensionPackageError::Manifest(
            "extension is already installed; use update for a different version".to_owned(),
        )),
        PackageOperation::Update if package < current => Err(ExtensionPackageError::Manifest(
            format!("update cannot downgrade extension from {current} to {package}"),
        )),
        PackageOperation::Install | PackageOperation::Update => Ok(()),
    }
}

fn recover_package_artifacts(paths: &NanikaPaths) -> Result<(), ExtensionPackageError> {
    let extensions_root = paths.app_data_root().join("extensions");
    fs::create_dir_all(&extensions_root)?;
    let Some(transaction) = PackageTransaction::load(&extensions_root)? else {
        return Ok(());
    };
    transaction.validate()?;
    let extension_root = extensions_root.join(&transaction.extension_id);
    validate_managed_path(&extensions_root, &extension_root)?;
    match transaction.operation.as_str() {
        "replace" => {
            let version = transaction.version.as_deref().ok_or_else(|| {
                ExtensionPackageError::Manifest(
                    "replacement recovery journal has no version".to_owned(),
                )
            })?;
            let version_root = extension_root.join(version);
            let backup_root = extension_root.join(&transaction.backup_name);
            validate_managed_path(&extension_root, &version_root)?;
            validate_managed_path(&extension_root, &backup_root)?;
            let expected_prefix = format!(".replaced-{version}-");
            if !transaction.backup_name.starts_with(&expected_prefix) {
                return Err(ExtensionPackageError::Manifest(
                    "replacement recovery journal has an unexpected backup name".to_owned(),
                ));
            }
            if version_root.exists() {
                if backup_root.exists() {
                    fs::remove_dir_all(backup_root)?;
                }
            } else if backup_root.exists() {
                fs::rename(backup_root, version_root)?;
            } else {
                return Err(ExtensionPackageError::Manifest(
                    "replacement recovery artifacts are missing".to_owned(),
                ));
            }
        }
        "remove" => {
            let backup_root = extensions_root.join(&transaction.backup_name);
            validate_managed_path(&extensions_root, &backup_root)?;
            let expected_prefix = format!(".removed-{}-", transaction.extension_id);
            if !transaction.backup_name.starts_with(&expected_prefix) {
                return Err(ExtensionPackageError::Manifest(
                    "removal recovery journal has an unexpected backup name".to_owned(),
                ));
            }
            if backup_root.exists() {
                let installed = HostDatabase::open(paths.host_database())?
                    .extension(&transaction.extension_id)?
                    .is_some();
                if installed && !extension_root.exists() {
                    fs::rename(backup_root, extension_root)?;
                } else {
                    fs::remove_dir_all(backup_root)?;
                }
            }
        }
        _ => unreachable!("validated package transaction operation"),
    }
    PackageTransaction::clear(&extensions_root)
}

fn validate_package_path(path: &Path) -> Result<(), ExtensionPackageError> {
    if path.extension().and_then(|value| value.to_str()) != Some("nanika") {
        return Err(ExtensionPackageError::Manifest(
            "extension package must use the .nanika suffix".to_owned(),
        ));
    }
    let size = fs::metadata(path)?.len();
    if size == 0 || size > MAX_PACKAGE_BYTES {
        return Err(ExtensionPackageError::Manifest(format!(
            "package size must be between 1 and {MAX_PACKAGE_BYTES} bytes"
        )));
    }
    Ok(())
}

fn extract_package(path: &Path, destination: &Path) -> Result<(), ExtensionPackageError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    if archive.is_empty() || archive.len() > MAX_ENTRIES {
        return Err(ExtensionPackageError::Manifest(format!(
            "package must contain between 1 and {MAX_ENTRIES} entries"
        )));
    }
    let mut expanded = 0_u64;
    let mut names = HashSet::new();
    let mut directories = HashSet::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(ExtensionPackageError::Manifest(
                "package uses an unsupported compression method".to_owned(),
            ));
        }
        if entry.size() > 1024 * 1024
            && entry.size()
                > entry
                    .compressed_size()
                    .max(1)
                    .saturating_mul(MAX_COMPRESSION_RATIO)
        {
            return Err(ExtensionPackageError::Manifest(
                "package entry exceeds the compression ratio limit".to_owned(),
            ));
        }
        expanded = expanded.saturating_add(entry.size());
        if expanded > MAX_EXPANDED_BYTES {
            return Err(ExtensionPackageError::Manifest(
                "package exceeds the expanded size limit".to_owned(),
            ));
        }
        let relative = entry.enclosed_name().ok_or_else(|| {
            ExtensionPackageError::Manifest("package contains an unsafe path".to_owned())
        })?;
        validate_relative_path(&relative)?;
        let collision_key = relative
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        if !names.insert(collision_key) {
            return Err(ExtensionPackageError::Manifest(
                "package contains colliding paths".to_owned(),
            ));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(ExtensionPackageError::Manifest(
                "package symlinks are not allowed".to_owned(),
            ));
        }
        let output = destination.join(&relative);
        if entry.is_dir() {
            ensure_package_directories(destination, &relative, &mut directories)?;
            continue;
        }
        if let Some(parent) = relative.parent() {
            ensure_package_directories(destination, parent, &mut directories)?;
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ExtensionPackageError::Manifest(
                        "package contains filesystem-colliding paths".to_owned(),
                    )
                } else {
                    error.into()
                }
            })?;
        std::io::copy(&mut entry, &mut file)?;
        file.flush()?;
    }
    Ok(())
}

fn ensure_package_directories(
    root: &Path,
    relative: &Path,
    directories: &mut HashSet<PathBuf>,
) -> Result<(), ExtensionPackageError> {
    let mut current = PathBuf::new();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(ExtensionPackageError::Manifest(
                "package contains an unsafe directory path".to_owned(),
            ));
        };
        current.push(component);
        if directories.insert(current.clone()) {
            fs::create_dir(root.join(&current)).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ExtensionPackageError::Manifest(
                        "package contains filesystem-colliding paths".to_owned(),
                    )
                } else {
                    error.into()
                }
            })?;
        }
    }
    Ok(())
}

fn load_manifest(path: PathBuf) -> Result<ExtensionManifest, ExtensionPackageError> {
    let metadata = fs::metadata(&path)?;
    if metadata.len() == 0 || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(ExtensionPackageError::Manifest(
            "manifest size is invalid".to_owned(),
        ));
    }
    let text = fs::read_to_string(path)?;
    parse_to_serde_value(&text, &ParseOptions::default())
        .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))
}

fn validate_manifest(manifest: &ExtensionManifest) -> Result<(), ExtensionPackageError> {
    if manifest.format != MANIFEST_FORMAT || manifest.manifest_version != MANIFEST_VERSION {
        return Err(ExtensionPackageError::Manifest(
            "unsupported manifest format".to_owned(),
        ));
    }
    manifest
        .runtime
        .validate()
        .map_err(ExtensionPackageError::Manifest)?;
    if !nanika_storage::is_valid_extension_id(&manifest.id) {
        return Err(ExtensionPackageError::Manifest(
            "invalid extension id".to_owned(),
        ));
    }
    if nanika_core::BUILTIN_EXTENSION_IDS.contains(&manifest.id.as_str()) {
        return Err(ExtensionPackageError::Manifest(
            "external packages cannot use a built-in extension id".to_owned(),
        ));
    }
    let _ = Version::parse(&manifest.version)
        .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))?;
    let host_requirement = VersionReq::parse(&manifest.host_api)
        .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))?;
    let host_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| ExtensionPackageError::Manifest(error.to_string()))?;
    if !host_requirement.matches(&host_version) {
        return Err(ExtensionPackageError::Manifest(format!(
            "host API requirement {} does not match {}",
            manifest.host_api, host_version
        )));
    }
    if manifest.targets.is_empty() {
        return Err(ExtensionPackageError::Manifest(
            "manifest has no target entrypoints".to_owned(),
        ));
    }
    for (target, entrypoint) in &manifest.targets {
        if !matches!(
            target.as_str(),
            "x86_64-pc-windows-msvc" | "aarch64-apple-darwin" | "x86_64-apple-darwin"
        ) {
            return Err(ExtensionPackageError::Manifest(format!(
                "unsupported extension target: {target}"
            )));
        }
        validate_target_entrypoint(target, entrypoint)?;
    }
    if !manifest.capabilities.is_empty() {
        return Err(ExtensionPackageError::Manifest(
            "extension capabilities are reserved for a future manifest version".to_owned(),
        ));
    }
    let mut permissions = HashSet::new();
    for permission in &manifest.permissions {
        if !matches!(permission.as_str(), "process.launch" | "clipboard.write") {
            return Err(ExtensionPackageError::Manifest(format!(
                "unsupported extension permission: {permission}"
            )));
        }
        if !permissions.insert(permission) {
            return Err(ExtensionPackageError::Manifest(format!(
                "duplicate extension permission: {permission}"
            )));
        }
    }
    let mut dependencies = HashSet::new();
    for dependency in &manifest.dependencies {
        if !nanika_storage::is_valid_extension_id(dependency) {
            return Err(ExtensionPackageError::Manifest(format!(
                "invalid extension dependency: {dependency}"
            )));
        }
        if dependency == &manifest.id {
            return Err(ExtensionPackageError::Manifest(
                "an extension cannot depend on itself".to_owned(),
            ));
        }
        if !dependencies.insert(dependency) {
            return Err(ExtensionPackageError::Manifest(format!(
                "duplicate extension dependency: {dependency}"
            )));
        }
    }
    if !manifest.dependencies.is_empty() {
        return Err(ExtensionPackageError::Manifest(
            "extension dependencies require a future dependency resolver".to_owned(),
        ));
    }
    if !manifest.activation_events.is_empty() {
        return Err(ExtensionPackageError::Manifest(
            "extension activation events are reserved for a future manifest version".to_owned(),
        ));
    }
    if !manifest.contributions.is_null()
        && !manifest
            .contributions
            .as_object()
            .is_some_and(|contributions| contributions.is_empty())
    {
        return Err(ExtensionPackageError::Manifest(
            "extension manifest contributions are reserved for a future manifest version"
                .to_owned(),
        ));
    }
    Ok(())
}

fn target_entrypoint(
    manifest: &ExtensionManifest,
    target: &str,
) -> Result<PathBuf, ExtensionPackageError> {
    let target_entrypoint = manifest.targets.get(target).ok_or_else(|| {
        ExtensionPackageError::Manifest(format!("package does not support target {target}"))
    })?;
    validate_target_entrypoint(target, target_entrypoint)?;
    Ok(PathBuf::from(&target_entrypoint.entrypoint))
}

fn validate_target_entrypoint(
    target: &str,
    target_entrypoint: &ExtensionTarget,
) -> Result<(), ExtensionPackageError> {
    let ExtensionTarget { entrypoint } = target_entrypoint;
    let path = PathBuf::from(entrypoint);
    validate_relative_path(&path)?;
    let expected = Path::new("bin").join(target);
    let relative_entrypoint = path.strip_prefix(&expected).map_err(|_| {
        ExtensionPackageError::Manifest("entrypoint must be under bin/<target>".to_owned())
    })?;
    if relative_entrypoint.as_os_str().is_empty() {
        return Err(ExtensionPackageError::Manifest(
            "entrypoint must be under bin/<target>".to_owned(),
        ));
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), ExtensionPackageError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ExtensionPackageError::Manifest(
            "package path is not a safe relative path".to_owned(),
        ));
    }
    Ok(())
}

fn validate_managed_path(root: &Path, path: &Path) -> Result<(), ExtensionPackageError> {
    if path == root || !path.starts_with(root) {
        return Err(ExtensionPackageError::Manifest(
            "extension path escaped its managed root".to_owned(),
        ));
    }
    Ok(())
}

fn prepare_extension_root(
    extensions_root: &Path,
    extension_id: &str,
) -> Result<PathBuf, ExtensionPackageError> {
    let canonical_root = fs::canonicalize(extensions_root)?;
    let extension_root = extensions_root.join(extension_id);
    if extension_root.exists() {
        let metadata = fs::symlink_metadata(&extension_root)?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(ExtensionPackageError::Manifest(
                "extension root is not a managed directory".to_owned(),
            ));
        }
    } else {
        fs::create_dir(&extension_root)?;
    }
    let canonical_extension = fs::canonicalize(&extension_root)?;
    validate_managed_path(&canonical_root, &canonical_extension)?;
    Ok(extension_root)
}

fn validate_version_directory(
    extensions_root: &Path,
    version_root: &Path,
) -> Result<(), ExtensionPackageError> {
    let metadata = fs::symlink_metadata(version_root)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(ExtensionPackageError::Manifest(
            "extension version is not a managed directory".to_owned(),
        ));
    }
    let canonical_root = fs::canonicalize(extensions_root)?;
    let canonical_version = fs::canonicalize(version_root)?;
    validate_managed_path(&canonical_root, &canonical_version)?;
    let digest = version_root.join(".package.sha256");
    let digest_metadata = fs::symlink_metadata(&digest)?;
    if !digest_metadata.file_type().is_file() || digest_metadata.file_type().is_symlink() {
        return Err(ExtensionPackageError::Manifest(
            "extension digest marker is not a managed file".to_owned(),
        ));
    }
    Ok(())
}

fn rollback_version(version_root: &Path, replaced_root: Option<&Path>) -> std::io::Result<()> {
    if version_root.exists() {
        fs::remove_dir_all(version_root)?;
    }
    if let Some(replaced) = replaced_root {
        fs::rename(replaced, version_root)?;
    }
    Ok(())
}

fn restore_registry(
    store: &ConfigStore,
    registry: &ExtensionRegistryConfig,
    existed: bool,
) -> Result<(), String> {
    if existed {
        registry.save(store)
    } else {
        match fs::remove_file(store.extensions_file()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

fn current_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        _ => "unsupported",
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
