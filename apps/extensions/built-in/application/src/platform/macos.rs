#![allow(unsafe_code)]
use crate::ApplicationEntryData;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use objc2_foundation::{NSBundle, NSFileManager, NSLocale, NSString};
use plist::Value;

mod icons;

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
    enabled: impl Fn(&str) -> bool,
) -> Result<DiscoveryRoots, ApplicationError> {
    let mut roots = DiscoveryRoots::default();
    if enabled(SYSTEM_APPLICATIONS_KEY) {
        roots.paths.extend(
            [SYSTEM_APPLICATION_ROOT, SYSTEM_APPLICATIONS_ROOT]
                .into_iter()
                .map(PathBuf::from),
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
                        "HOME must be an absolute path".to_owned(),
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
    state: &mut DiscoveryState,
    path: &Path,
    priority: usize,
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
        .or_else(|| string_value(dictionary.get("CFBundleName")))
        .map(str::to_owned)
        .or_else(|| {
            path.file_stem()
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let display_name = localized_display_name(path, state.preferred_languages(preferred_languages))
        .unwrap_or_else(|| bundle_display_name.clone());
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
                .unwrap_or_default(),
        ],
    );
    let arguments_json = ApplicationArguments::empty().to_json()?;
    Ok(Some(ApplicationEntry::new(ApplicationEntryData {
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens,
        launch_kind: "macos-bundle".to_owned(),
        target_path: path.to_string_lossy().into_owned(),
        arguments_json,
        icon_key: String::new(),
        icon_source: Some(path.to_path_buf()),
        icon_index: 0,
        priority,
    })))
}

fn localized_display_name(path: &Path, localizations: &[String]) -> Option<String> {
    let bundle_path = path;
    let path = NSString::from_str(&bundle_path.to_string_lossy());
    let key = NSString::from_str("CFBundleDisplayName");
    let bundle = NSBundle::bundleWithPath(&path);
    let display_name = loctable_display_name(bundle_path, localizations)
        .or_else(|| {
            bundle
                .and_then(|bundle| bundle.objectForInfoDictionaryKey(&key))
                .and_then(|value| value.downcast::<NSString>().ok())
                .map(|value| value.to_string())
        })
        .unwrap_or_else(|| {
            NSFileManager::defaultManager()
                .displayNameAtPath(&path)
                .to_string()
        });
    let display_name = display_name.trim();
    let display_name = display_name
        .strip_suffix(".app")
        .or_else(|| display_name.strip_suffix(".APP"))
        .unwrap_or(display_name)
        .trim();
    (!display_name.is_empty()).then(|| display_name.to_owned())
}

fn preferred_languages() -> Vec<String> {
    NSLocale::preferredLanguages()
        .into_iter()
        .map(|localization| localization.to_string())
        .collect()
}

fn loctable_display_name(bundle: &Path, localizations: &[String]) -> Option<String> {
    let file = fs::File::open(bundle.join("Contents/Resources/InfoPlist.loctable")).ok()?;
    let table = Value::from_reader(file).ok()?;
    let table = table.as_dictionary()?;
    for localization in localizations {
        for localization in localization_candidates(localization) {
            let value = table
                .get(&localization)
                .and_then(Value::as_dictionary)
                .and_then(|values| values.get("CFBundleDisplayName"))
                .and_then(Value::as_string)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if let Some(value) = value {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn localization_candidates(localization: &str) -> Vec<String> {
    let canonical = localization.replace('-', "_");
    let lower = canonical.to_ascii_lowercase();
    let mut candidates = vec![localization.to_owned(), canonical.clone()];
    if lower.starts_with("zh_hans") {
        candidates.push("zh_CN".to_owned());
    } else if lower.starts_with("zh_hant_hk") {
        candidates.push("zh_HK".to_owned());
    } else if lower.starts_with("zh_hant") {
        candidates.push("zh_TW".to_owned());
    }
    if let Some((language, _)) = canonical.split_once('_') {
        candidates.push(language.to_owned());
    }
    candidates.dedup();
    candidates
}

fn normalized_aliases<'a>(
    normalized_name: &str,
    aliases: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut normalized = Vec::<String>::new();
    for alias in aliases {
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
        "Windows shortcut targets are unavailable on this platform".to_owned(),
    ))
}
