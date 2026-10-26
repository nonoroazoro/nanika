#![allow(unsafe_code)]
use crate::ApplicationEntryData;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use plist::Value;

mod icons;
mod names;

pub(super) use icons::{extract_icons, icon_cache_key};

use super::DiscoveryRoots;
use crate::normalization::{normalize_name, path_key, stable_hash};
use crate::{ApplicationArguments, ApplicationEntry, ApplicationError, DiscoveryState};

const SYSTEM_APPLICATION_ROOT: &str = "/Applications";
const SYSTEM_APPLICATIONS_ROOT: &str = "/System/Applications";
const USER_APPLICATION_DIRECTORY: &str = "Applications";
const HOME_ENVIRONMENT: &str = "HOME";
const SYSTEM_APPLICATIONS_KEY: &str = "application.builtin.macos.systemApplications";
const USER_APPLICATIONS_KEY: &str = "application.builtin.macos.userApplications";

pub(super) fn standard_roots(
    enabled: impl Fn(&str) -> bool
) -> Result<DiscoveryRoots, ApplicationError> {
    let mut roots = DiscoveryRoots::default();
    if enabled(SYSTEM_APPLICATIONS_KEY) {
        roots.paths.extend(
            [SYSTEM_APPLICATION_ROOT, SYSTEM_APPLICATIONS_ROOT]
                .into_iter()
                .map(PathBuf::from)
        );
    }
    if enabled(USER_APPLICATIONS_KEY) {
        let user_root = std::env::var_os(HOME_ENVIRONMENT)
            .map(PathBuf::from)
            .ok_or_else(|| ApplicationError::Configuration("HOME is not set".to_owned()))
            .and_then(|home| {
                if home.is_absolute() {
                    Ok(Some(home.join(USER_APPLICATION_DIRECTORY)))
                } else {
                    Err(ApplicationError::Configuration(
                        "HOME must be an absolute path".to_owned()
                    ))
                }
            });
        roots.include(USER_APPLICATIONS_KEY, user_root);
    }
    Ok(roots)
}

pub(super) fn is_application_path(_path: &Path) -> bool {
    false
}

pub(super) fn is_application_bundle(path: &Path) -> bool {
    path.is_dir()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

pub(super) fn read_entry(
    _state: &mut DiscoveryState,
    path: &Path,
    priority: usize
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    if !is_application_bundle(path) {
        return Ok(None);
    }
    let info_file = fs::File::open(path.join("Contents/Info.plist"))?;
    let info =
        Value::from_reader(info_file).map_err(|error| std::io::Error::other(error.to_string()))?;
    let Some(dictionary) = info.as_dictionary() else {
        return Ok(None);
    };
    let Some(executable_name) = string_value(dictionary.get("CFBundleExecutable")) else {
        return Ok(None);
    };
    let executable = path.join("Contents/MacOS").join(executable_name);
    let executable_metadata = executable.metadata()?;
    if !executable_metadata.is_file() || executable_metadata.permissions().mode() & 0o111 == 0 {
        return Ok(None);
    }
    let bundle_id = string_value(dictionary.get("CFBundleIdentifier")).map(str::to_owned);
    let bundle_display_name = string_value(dictionary.get("CFBundleDisplayName"))
        .and_then(names::normalize)
        .or_else(|| string_value(dictionary.get("CFBundleName")).and_then(names::normalize))
        .map(str::to_owned)
        .unwrap_or_default();
    let (display_name, localized_names) = names::read(
        path,
        &bundle_display_name,
        string_value(dictionary.get("CFBundleDevelopmentRegion"))
    );
    let executable_key = path_key(&executable);
    let identity_source = bundle_id.as_deref().unwrap_or(&executable_key);
    let identity = stable_hash(&["macos", identity_source]);
    let normalized_name = normalize_name(&display_name);
    let normalized_tokens = normalized_aliases(
        &normalized_name,
        [
            bundle_display_name.as_str(),
            string_value(dictionary.get("CFBundleName")).unwrap_or_default(),
            path.file_stem()
                .map(|value| value.to_string_lossy())
                .as_deref()
                .unwrap_or_default()
        ]
    );
    let arguments_json = ApplicationArguments::empty().to_json()?;
    Ok(Some(ApplicationEntry::new(ApplicationEntryData {
        localized_names,
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens,
        launch_kind: "macos-bundle".to_owned(),
        target_path: path.to_string_lossy().into_owned(),
        arguments_json,
        icon_key: String::new(),
        icon_source: Some(crate::ApplicationIconSource::File {
            path: path.to_path_buf(),
            index: 0
        }),
        priority
    })))
}

fn normalized_aliases<'a>(
    normalized_name: &str,
    aliases: impl IntoIterator<Item = &'a str>
) -> String {
    let mut normalized = Vec::<String>::new();
    for alias in aliases {
        let Some(alias) = names::normalize(alias) else {
            continue;
        };
        let alias = normalize_name(alias);
        if alias.is_empty()
            || alias == normalized_name
            || normalized.iter().any(|existing| existing == &alias)
        {
            continue;
        }
        normalized.push(alias);
    }
    normalized.join("\n")
}

fn string_value(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_string)
}

pub(super) fn shortcut_target(_path: &Path) -> Result<String, ApplicationError> {
    Err(ApplicationError::Configuration(
        "Windows shortcut targets are unavailable on this platform".to_owned()
    ))
}

pub(super) fn inventories(
    _enabled: &std::collections::BTreeSet<String>,
    _cancelled: &mut dyn FnMut() -> bool
) -> Vec<super::DiscoveryInventory> {
    Vec::new()
}
