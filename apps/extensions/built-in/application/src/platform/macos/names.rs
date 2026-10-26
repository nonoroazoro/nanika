use objc2::rc::Retained;
use objc2_foundation::{
    NSData, NSDictionary, NSLocale, NSPropertyListMutabilityOptions, NSPropertyListSerialization,
    NSString
};
use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const MAX_METADATA_BYTES: u64 = 2 * 1024 * 1024;

/// Retain source names independently of supported UI locales and the process language.
pub(super) fn read(
    path: &Path,
    fallback: &str,
    development_locale: Option<&str>
) -> (String, BTreeMap<String, String>) {
    // Discovery runs off the UI thread, without its event loop autorelease pool.
    objc2::rc::autoreleasepool(|_| {
        let resources = path.join("Contents/Resources");
        let mut names = BTreeMap::new();
        let mut base_name = None;
        let table_path = resources.join("InfoPlist.loctable");
        match _read(&table_path).and_then(|bytes| {
            plist::Value::from_reader(std::io::Cursor::new(bytes)).map_err(std::io::Error::other)
        }) {
            Ok(table) => {
                if let Some(table) = table.as_dictionary() {
                    for (locale, value) in table {
                        let Some(name) = value.as_dictionary().and_then(|value| {
                            ["CFBundleDisplayName", "CFBundleName"]
                                .into_iter()
                                .find_map(|key| {
                                    value
                                        .get(key)
                                        .and_then(plist::Value::as_string)
                                        .and_then(normalize)
                                        .map(str::to_owned)
                                })
                        }) else {
                            continue;
                        };
                        if locale == "Base" {
                            base_name = Some(name);
                        } else if let Some(locale) = _locale(locale) {
                            names.insert(locale, name);
                        }
                    }
                }
            }
            Err(error) => _report(&table_path, &error)
        }
        // Read actual resources on every scan; NSBundle caches metadata across application updates.
        let mut available = Vec::new();
        match std::fs::read_dir(&resources) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.extension().is_some_and(|value| value == "lproj") {
                                available.push(path);
                            }
                        }
                        Err(error) => _report(&resources, &error)
                    }
                }
            }
            Err(error) => _report(&resources, &error)
        }
        available.sort_unstable();
        for directory in available {
            let Some(source) = directory.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let locale = _locale(source);
            if source == "Base" {
                if base_name.is_some() {
                    continue;
                }
            } else if locale
                .as_ref()
                .is_none_or(|locale| names.contains_key(locale))
            {
                continue;
            }
            let file = directory.join("InfoPlist.strings");
            match _strings_name(&file) {
                Ok(Some(name)) => {
                    if source == "Base" {
                        base_name = Some(name);
                    } else if let Some(locale) = locale {
                        names.insert(locale, name);
                    }
                }
                Ok(None) => {}
                Err(error) => _report(&file, &error)
            }
        }
        let source_name = base_name.or_else(|| normalize(fallback).map(str::to_owned));
        // A source fallback must not outrank an actual translation of the same language.
        // It remains searchable through TranslatedText::values even when not inserted here.
        if let Some(locale) = development_locale.and_then(_locale)
            && !names
                .keys()
                .any(|tag| tag.split('-').next() == locale.split('-').next())
            && let Some(name) = &source_name
        {
            names.entry(locale).or_insert_with(|| name.clone());
        }
        // A missing or invalid default must not poison valid translations or the source transaction.
        // Only declared source text is attributed to development_locale, never this inferred fallback.
        let fallback = source_name
            .or_else(|| names.values().next().cloned())
            .unwrap_or_else(|| _file_name(path));
        let text = nanika_protocol::TranslatedText {
            fallback,
            translations: names
        };
        if !text.translations.is_empty()
            && let Err(error) = text.validate()
        {
            // Invalid optional metadata must not prevent the application itself from being published.
            _report(&resources, &std::io::Error::other(error));
            return (text.fallback, BTreeMap::new());
        }
        (text.fallback, text.translations)
    })
}

/// Apply the same limits to native titles, source fallbacks and searchable aliases.
pub(super) fn normalize(value: &str) -> Option<&str> {
    let value = value.trim();
    let value = value
        .strip_suffix(".app")
        .or_else(|| value.strip_suffix(".APP"))
        .unwrap_or(value)
        .trim();
    (!value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control))
        .then_some(value)
}

fn _file_name(path: &Path) -> String {
    let value = path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("Application"))
        .to_string_lossy();
    // Even a bundle filename containing controls needs a nonempty, bounded display label.
    normalize(&value)
        .map(str::to_owned)
        .unwrap_or_else(|| value.escape_default().take(1024).collect())
}

fn _strings_name(path: &Path) -> std::io::Result<Option<String>> {
    // Drain Foundation temporaries per file, not after every localization in the bundle.
    objc2::rc::autoreleasepool(|_| {
        let bytes = _read(path)?;
        let data = NSData::with_bytes(&bytes);
        // Foundation handles binary, XML and UTF-16/OpenStep .strings files. No output pointer is supplied.
        let value = unsafe {
            NSPropertyListSerialization::propertyListWithData_options_format_error(
                &data,
                NSPropertyListMutabilityOptions::Immutable,
                std::ptr::null_mut()
            )
        }
        .map_err(|error| std::io::Error::other(error.to_string()))?;
        let dictionary: Retained<NSDictionary> = value
            .downcast()
            .map_err(|_| std::io::Error::other("InfoPlist.strings is not a dictionary"))?;
        Ok(["CFBundleDisplayName", "CFBundleName"]
            .into_iter()
            .find_map(|key| {
                dictionary
                    .objectForKey(&NSString::from_str(key))
                    .and_then(|value| value.downcast::<NSString>().ok())
                    .and_then(|value| normalize(&value.to_string()).map(str::to_owned))
            }))
    })
}

fn _read(path: &Path) -> std::io::Result<Vec<u8>> {
    // A FIFO can block during open, before metadata is available. Check the opened handle
    // after a nonblocking open so replacement of the path cannot bypass the type check.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("metadata is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_METADATA_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err(std::io::Error::other(
            "application name metadata exceeds 2 MiB"
        ));
    }
    Ok(bytes)
}

fn _locale(value: &str) -> Option<String> {
    if value == "Base" {
        return None;
    }
    let normalized = value.replace('_', "-");
    let language = normalized.split('-').next()?;
    let normalized = if language.len() > 3 {
        // Foundation recognizes legacy bundle directory names such as English and Japanese.
        NSLocale::canonicalLanguageIdentifierFromString(&NSString::from_str(value)).to_string()
    } else {
        normalized
    };
    nanika_protocol::canonical_language_tag(&normalized).filter(|locale| locale.len() <= 64)
}

fn _report(path: &Path, error: &std::io::Error) {
    if error.kind() != std::io::ErrorKind::NotFound {
        eprintln!(
            "application name metadata failed for {}: {error}",
            path.display()
        );
    }
}

#[cfg(test)]
#[path = "../../../tests/platform/macos/names.rs"]
mod tests;
