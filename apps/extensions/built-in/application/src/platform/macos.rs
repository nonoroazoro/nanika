#![allow(unsafe_code)]

use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use icns::{IconFamily, PixelFormat};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
use objc2_core_foundation::CFData;
use objc2_foundation::{NSBundle, NSData, NSDictionary, NSFileManager, NSLocale, NSString};
use plist::Value;

use crate::normalization::{normalize_name, path_key, stable_hash};
use crate::{ApplicationArguments, ApplicationEntry, ApplicationError, DiscoveryState};

pub(super) fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let user_applications = PathBuf::from(home).join("Applications");
        if user_applications.is_dir() {
            roots.push(user_applications);
        }
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
    seen_at: u64,
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
    let icon_source = string_value(dictionary.get("CFBundleIconFile"))
        .map(|name| {
            let mut name = PathBuf::from(name);
            if name.extension().is_none() {
                name.set_extension("icns");
            }
            path.join("Contents/Resources").join(name)
        })
        .filter(|path| path.is_file());
    Ok(Some(ApplicationEntry {
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens,
        launch_kind: "macos-bundle".to_owned(),
        target_path: path.to_string_lossy().into_owned(),
        working_directory: None,
        arguments_json,
        bundle_id,
        icon_key: String::new(),
        file_identity: executable_key,
        last_seen_at: seen_at,
        stale: false,
        icon_source,
        icon_index: 0,
        priority,
    }))
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

pub(super) fn extract_icon(
    source: &Path,
    _icon_index: i32,
    size: u32,
    target: &Path,
) -> Result<(), ApplicationError> {
    let pixels = match icns_icon(source, size) {
        Ok(pixels) => pixels,
        Err(icns_error) => {
            let Some(bundle) = application_bundle(source) else {
                return Err(icns_error);
            };
            match workspace_icon(&bundle, size) {
                Ok(pixels) => pixels,
                Err(_) => return Err(icns_error),
            }
        }
    };
    crate::icon_cache::write_png(target, size, size, &pixels)
}

fn icns_icon(source: &Path, size: u32) -> Result<Vec<u8>, ApplicationError> {
    let family = IconFamily::read(fs::File::open(source)?)?;
    let mut types = family.available_icons();
    types.sort_by_key(|icon_type| icon_type.pixel_width().abs_diff(size));
    types
        .into_iter()
        .filter_map(|icon_type| family.get_icon_with_type(icon_type).ok())
        .find_map(|image| {
            let image = image.convert_to(PixelFormat::RGBA);
            crate::normalize_icon_rgba(image.data(), image.width(), image.height(), size)
        })
        .ok_or_else(|| std::io::Error::other("ICNS contains no visible decodable icon").into())
}

fn application_bundle(source: &Path) -> Option<PathBuf> {
    source
        .ancestors()
        .find(|ancestor| {
            ancestor
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        })
        .map(Path::to_path_buf)
}

fn workspace_icon(bundle: &Path, size: u32) -> Result<Vec<u8>, ApplicationError> {
    let path = NSString::from_str(&bundle.to_string_lossy());
    let image = NSWorkspace::sharedWorkspace().iconForFile(&path);
    let tiff = image
        .TIFFRepresentation()
        .ok_or_else(|| std::io::Error::other("NSWorkspace icon has no TIFF representation"))?;
    let bitmap = NSBitmapImageRep::imageRepWithData(&tiff)
        .ok_or_else(|| std::io::Error::other("NSWorkspace icon TIFF could not decode"))?;
    let properties = NSDictionary::new();
    let png = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
    }
    .ok_or_else(|| std::io::Error::other("NSWorkspace icon could not encode as PNG"))?;
    let ns_data: &NSData = &png;
    let data: &CFData = ns_data.as_ref();
    let bytes = unsafe { data.as_bytes_unchecked() };
    let (pixels, width, height) = decode_png_rgba(bytes)?;
    crate::normalize_icon_rgba(&pixels, width, height, size)
        .ok_or_else(|| std::io::Error::other("NSWorkspace provided an empty application icon"))
        .map_err(Into::into)
}

fn decode_png_rgba(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), ApplicationError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(std::io::Error::other)?;
    let output_size = reader
        .output_buffer_size()
        .ok_or_else(|| std::io::Error::other("NSWorkspace PNG output size is unavailable"))?;
    let mut decoded = vec![0_u8; output_size];
    let output = reader
        .next_frame(&mut decoded)
        .map_err(std::io::Error::other)?;
    let decoded = &decoded[..output.buffer_size()];
    let pixels = match output.color_type {
        png::ColorType::Rgba => decoded.to_vec(),
        png::ColorType::Rgb => decoded
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX])
            .collect(),
        _ => {
            return Err(std::io::Error::other("NSWorkspace PNG color type is unsupported").into());
        }
    };
    Ok((pixels, output.width, output.height))
}

fn string_value(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_string)
}
