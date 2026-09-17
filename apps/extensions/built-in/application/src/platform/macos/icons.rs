#![allow(unsafe_code)]

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use std::sync::OnceLock;

use objc2_foundation::NSProcessInfo;
use plist::Value;

use crate::normalization::{path_key, stable_hash, timestamp_nanos};
use crate::{ApplicationError, DiscoveryState};

const WORKING_SIZE: usize = 256;
const RENDER_VERSION: &str = "macos-workspace-srgb-v1";

pub(crate) fn icon_cache_key(
    bundle: &Path,
    _icon_index: i32,
    state: &mut DiscoveryState,
) -> Result<String, ApplicationError> {
    static OS_VERSION: OnceLock<String> = OnceLock::new();
    let os_version = OS_VERSION.get_or_init(|| {
        NSProcessInfo::processInfo()
            .operatingSystemVersionString()
            .to_string()
    });
    let info_path = bundle.join("Contents/Info.plist");
    let info = Value::from_reader(fs::File::open(&info_path)?)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let dictionary = info
        .as_dictionary()
        .ok_or_else(|| std::io::Error::other("application Info.plist is not a dictionary"))?;
    let mut stamps = vec![RENDER_VERSION.to_owned(), os_version.clone()];
    append_stamp(&mut stamps, state, bundle)?;
    append_stamp(&mut stamps, state, &info_path)?;
    let contents = bundle.join("Contents");
    let resources = contents.join("Resources");
    for path in [
        contents.clone(),
        resources.clone(),
        resources.join("Assets.car"),
        bundle.join("Icon\r"),
    ] {
        append_optional_stamp(&mut stamps, state, &path)?;
    }
    if let Some(executable) = dictionary
        .get("CFBundleExecutable")
        .and_then(Value::as_string)
    {
        append_stamp(&mut stamps, state, &contents.join("MacOS").join(executable))?;
    }
    // This resource is only fingerprinted; NSWorkspace remains the sole image source.
    if let Some(name) = dictionary
        .get("CFBundleIconFile")
        .and_then(Value::as_string)
    {
        let path = Path::new(name);
        if path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            let mut path = resources.join(path);
            if path.extension().is_none() {
                path.set_extension("icns");
            }
            append_optional_stamp(&mut stamps, state, &path)?;
        }
    }
    Ok(stable_hash(
        &stamps.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
}

fn append_stamp(
    stamps: &mut Vec<String>,
    state: &mut DiscoveryState,
    path: &Path,
) -> Result<(), ApplicationError> {
    let metadata = state.metadata(path)?;
    stamps.push(crate::icon_cache::key_from_stamp(
        path,
        0,
        metadata.len(),
        timestamp_nanos(metadata.modified()?),
    ));
    // ctime catches Finder custom-icon xattrs; inode catches replacement with preserved timestamps.
    stamps.push(format!(
        "{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.ctime(),
        metadata.ctime_nsec()
    ));
    Ok(())
}

fn append_optional_stamp(
    stamps: &mut Vec<String>,
    state: &mut DiscoveryState,
    path: &Path,
) -> Result<(), ApplicationError> {
    match append_stamp(stamps, state, path) {
        Err(ApplicationError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            stamps.push(format!("missing:{}", path_key(path)));
            Ok(())
        }
        result => result,
    }
}

pub(crate) fn extract_icons(
    bundle: &Path,
    _icon_index: i32,
    sizes: &[u32],
    directory: &Path,
) -> Result<(), ApplicationError> {
    if sizes.is_empty() {
        return Ok(());
    }
    if !bundle.metadata()?.is_dir() {
        return Err(std::io::Error::other(format!(
            "application bundle is unavailable: {}",
            bundle.display()
        ))
        .into());
    }
    let pixels = nanika_platform::file_icon_pixels(bundle, 0, WORKING_SIZE as u32)?;
    for &size in sizes {
        let normalized =
            crate::normalize_icon_rgba(&pixels, WORKING_SIZE as u32, WORKING_SIZE as u32, size)
                .ok_or_else(|| {
                    std::io::Error::other("NSWorkspace provided an empty application icon")
                })?;
        crate::icon_cache::write_png(
            &directory.join(format!("{size}.png")),
            size,
            size,
            &normalized,
        )?;
    }
    Ok(())
}
