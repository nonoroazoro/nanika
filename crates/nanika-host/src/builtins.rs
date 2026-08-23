use std::ffi::OsString;
use std::path::{Path, PathBuf};

use nanika_config::ExtensionRegistryConfig;
pub(crate) use nanika_core::{
    APPLICATION_EXTENSION_ID, CALCULATOR_EXTENSION_ID, CLIPBOARD_EXTENSION_ID,
    COMMAND_EXTENSION_ID, SCRIPT_EXTENSION_ID,
};
use nanika_extension_package::resolve_active_extensions;
use nanika_storage::StoredExtension;
use nanika_storage::{ExtensionKind, NanikaPaths};

use crate::{BuiltinExtensionSpec, ExtensionProcess, PendingExtension};

const BUILTINS: [BuiltinExtensionSpec; 5] = [
    BuiltinExtensionSpec {
        extension_id: APPLICATION_EXTENSION_ID,
        binary_name: "nanika-extension-application",
        kind: ExtensionKind::BuiltIn,
        permissions: &["process.launch"],
    },
    BuiltinExtensionSpec {
        extension_id: COMMAND_EXTENSION_ID,
        binary_name: "nanika-extension-command",
        kind: ExtensionKind::BuiltIn,
        permissions: &["process.launch"],
    },
    BuiltinExtensionSpec {
        extension_id: SCRIPT_EXTENSION_ID,
        binary_name: "nanika-extension-script",
        kind: ExtensionKind::BuiltIn,
        permissions: &["process.launch"],
    },
    BuiltinExtensionSpec {
        extension_id: CALCULATOR_EXTENSION_ID,
        binary_name: "nanika-extension-calculator",
        kind: ExtensionKind::BuiltIn,
        permissions: &["clipboard.write"],
    },
    BuiltinExtensionSpec {
        extension_id: CLIPBOARD_EXTENSION_ID,
        binary_name: "nanika-extension-clipboard",
        kind: ExtensionKind::BuiltIn,
        permissions: &["clipboard.write"],
    },
];

pub(crate) fn spawn_extensions(
    paths: &NanikaPaths,
    registry: &ExtensionRegistryConfig,
    installed: &[StoredExtension],
) -> (Vec<PendingExtension>, Vec<String>) {
    let current_executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => return (Vec::new(), vec![error.to_string()]),
    };
    let mut extensions = Vec::with_capacity(BUILTINS.len());
    let mut errors = Vec::new();
    for spec in BUILTINS {
        if !registry.is_enabled(spec.extension_id, true) {
            continue;
        }
        let program = companion_executable(&current_executable, spec.binary_name);
        if !program.is_file() {
            errors.push(format!(
                "built-in extension executable is missing: {}",
                program.display()
            ));
            continue;
        }
        let arguments = [
            path_argument("data-root", paths.app_data_root()),
            path_argument("cache-root", paths.cache_root()),
            path_argument("config-root", paths.config_root()),
        ];
        match ExtensionProcess::spawn_with(&program, arguments, Default::default()) {
            Ok(process) => extensions.push(PendingExtension {
                extension_id: spec.extension_id.to_owned(),
                kind: spec.kind,
                permissions: spec
                    .permissions
                    .iter()
                    .map(|permission| (*permission).to_owned())
                    .collect(),
                process,
            }),
            Err(error) => errors.push(format!(
                "failed to spawn built-in extension {}: {error}",
                spec.extension_id
            )),
        }
    }
    let (external, external_errors) = resolve_active_extensions(paths, installed, registry);
    errors.extend(external_errors);
    for extension in external {
        let arguments = [
            path_argument("data-root", paths.app_data_root()),
            path_argument("cache-root", paths.cache_root()),
            path_argument("config-root", paths.config_root()),
        ];
        match ExtensionProcess::spawn_with(&extension.program, arguments, Default::default()) {
            Ok(process) => extensions.push(PendingExtension {
                extension_id: extension.extension_id,
                kind: ExtensionKind::External,
                permissions: extension.permissions,
                process,
            }),
            Err(error) => errors.push(format!(
                "failed to spawn external extension {}: {error}",
                extension.extension_id
            )),
        }
    }
    (extensions, errors)
}

fn companion_executable(current_executable: &Path, binary_name: &str) -> PathBuf {
    current_executable.with_file_name(format!("{binary_name}{}", std::env::consts::EXE_SUFFIX))
}

fn path_argument(name: &str, path: &Path) -> OsString {
    OsString::from(format!("--{name}={}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::companion_executable;
    use std::path::Path;

    #[test]
    fn builtins_resolve_next_to_the_host_executable() {
        let resolved = companion_executable(
            Path::new("C:/nanika/nanika-host.exe"),
            "nanika-extension-application",
        );
        assert_eq!(
            resolved,
            Path::new("C:/nanika").join(format!(
                "nanika-extension-application{}",
                std::env::consts::EXE_SUFFIX
            ))
        );
    }
}
