use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use icns::{IconFamily, PixelFormat};
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
    _state: &mut DiscoveryState,
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
    let display_name = string_value(dictionary.get("CFBundleDisplayName"))
        .or_else(|| string_value(dictionary.get("CFBundleName")))
        .map(str::to_owned)
        .or_else(|| {
            path.file_stem()
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let executable_key = path_key(&executable);
    let identity_source = bundle_id.as_deref().unwrap_or(&executable_key);
    let identity = stable_hash(&["macos", identity_source]);
    let normalized_name = normalize_name(&display_name);
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
        normalized_tokens: normalized_name,
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

pub(super) fn extract_icon(
    source: &Path,
    _icon_index: i32,
    size: u32,
    target: &Path,
) -> Result<(), ApplicationError> {
    let family = IconFamily::read(fs::File::open(source)?)?;
    let mut types = family.available_icons();
    types.sort_by_key(|icon_type| icon_type.pixel_width().abs_diff(size));
    let image = types
        .into_iter()
        .find_map(|icon_type| family.get_icon_with_type(icon_type).ok())
        .ok_or_else(|| std::io::Error::other("ICNS contains no decodable icon"))?;
    let image = image.convert_to(PixelFormat::RGBA);
    let pixels =
        crate::image_resize::resize_rgba(image.data(), image.width(), image.height(), size, size);
    crate::icon_cache::write_png(target, size, size, &pixels)
}

fn string_value(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_string)
}
