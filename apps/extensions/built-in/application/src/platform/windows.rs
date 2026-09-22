#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink};
use windows::core::{Interface, PCWSTR};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows_sys::Win32::UI::Shell::{
    FOLDERID_CommonPrograms, FOLDERID_Programs, KF_FLAG_DONT_VERIFY, SHGetKnownFolderPath,
};

use super::shell_link_metadata::ShellLinkMetadata;
use crate::normalization::{normalize_name, path_key, stable_hash, timestamp_nanos};
use crate::{ApplicationArguments, ApplicationEntry, ApplicationError, DiscoveryState};

pub(super) fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
    let roots: Vec<PathBuf> = [FOLDERID_Programs, FOLDERID_CommonPrograms]
        .iter()
        .map(known_folder)
        .collect::<Result<_, _>>()?;
    let mut roots = roots;
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA")
        && let Some(root) = packaged_root(PathBuf::from(local_app_data).join("Packages"))
    {
        roots.push(root);
    }
    if let Ok(program_files) = std::env::var("ProgramFiles")
        && let Some(root) = packaged_root(PathBuf::from(program_files).join("WindowsApps"))
    {
        roots.push(root);
    }
    Ok(roots)
}

fn packaged_root(root: PathBuf) -> Option<PathBuf> {
    let entries = std::fs::read_dir(&root).ok()?;
    if entries
        .filter_map(Result::ok)
        .any(|entry| entry.path().join("AppxManifest.xml").is_file())
    {
        Some(root)
    } else {
        None
    }
}

pub(super) fn is_application_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("lnk") || extension.eq_ignore_ascii_case("exe")
        })
}

pub(super) fn is_application_bundle(path: &Path) -> bool {
    path.is_dir() && path.join("AppxManifest.xml").is_file()
}

pub(super) fn read_entry(
    state: &mut DiscoveryState,
    path: &Path,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("lnk") {
        read_shell_link(state, path, priority)
    } else if extension.eq_ignore_ascii_case("exe") {
        read_executable(state, path, priority)
    } else if is_application_bundle(path) {
        read_packaged_application(path, priority)
    } else {
        Ok(None)
    }
}

fn read_packaged_application(
    package_root: &Path,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    let manifest_path = package_root.join("AppxManifest.xml");
    let manifest = std::fs::read_to_string(&manifest_path)?;
    let identity = xml_attribute(&manifest, "Identity", "Name").unwrap_or_else(|| {
        package_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_owned()
    });
    let application = xml_element(&manifest, "Application").ok_or_else(|| {
        ApplicationError::Configuration(format!(
            "packaged app manifest has no Application: {}",
            manifest_path.display()
        ))
    })?;
    let app_id =
        xml_attribute(application, "Application", "Id").unwrap_or_else(|| "App".to_owned());
    let display_name = xml_attribute(application, "Application", "DisplayName")
        .filter(|value| !value.starts_with("ms-resource:"))
        .unwrap_or_else(|| identity.clone());
    let visual = xml_element(application, "uap:VisualElements")
        .or_else(|| xml_element(application, "VisualElements"));
    let icon = visual
        .and_then(|value| xml_attribute(value, "uap:VisualElements", "Square44x44Logo"))
        .or_else(|| visual.and_then(|value| xml_attribute(value, "VisualElements", "Logo")))
        .map(|value| package_root.join(value));
    let package_id = format!("{identity}!{app_id}");
    let arguments_json =
        ApplicationArguments::from_windows_raw(Some(format!("shell:AppsFolder\\{package_id}")))
            .to_json()?;
    let entry_id = stable_hash(&["windows-packaged", &package_id, &path_key(package_root)]);
    let icon_key = icon.as_ref().map_or_else(String::new, |path| {
        crate::icon_cache::key_from_stamp(path, 0, 0, 0)
    });
    Ok(Some(ApplicationEntry {
        entry_id: format!("app.{entry_id}"),
        source_key: path_key(package_root),
        display_name: display_name.clone(),
        normalized_name: normalize_name(&display_name),
        normalized_tokens: normalize_name(&display_name),
        search_readings: Vec::new(),
        launch_kind: "windows-packaged".to_owned(),
        target_path: "explorer.exe".to_owned(),
        working_directory: None,
        arguments_json,
        bundle_id: Some(package_id),
        icon_key,
        icon_source: icon,
        icon_index: 0,
        priority,
    }))
}

fn xml_element<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{name}"))?;
    let rest = &xml[start..];
    let end = rest.find('>')?;
    Some(&rest[..=end])
}

fn xml_attribute(xml: &str, element: &str, attribute: &str) -> Option<String> {
    let element = xml_element(xml, element)?;
    let marker = format!("{attribute}=\"");
    let start = element.find(&marker)? + marker.len();
    let end = element[start..].find('\"')? + start;
    Some(element[start..end].replace('/', "\\"))
}

pub(super) fn icon_cache_key(
    source: &Path,
    icon_index: i32,
    state: &mut DiscoveryState,
) -> Result<String, ApplicationError> {
    let metadata = state.metadata(source)?;
    Ok(crate::icon_cache::key_from_stamp(
        source,
        icon_index,
        metadata.len(),
        crate::normalization::timestamp_nanos(metadata.modified()?),
    ))
}

pub(super) fn extract_icons(
    source: &Path,
    icon_index: i32,
    sizes: &[u32],
    directory: &Path,
) -> Result<(), ApplicationError> {
    for &size in sizes {
        extract_icon(
            source,
            icon_index,
            size,
            &directory.join(format!("{size}.png")),
        )?;
    }
    Ok(())
}

fn extract_icon(
    source: &Path,
    icon_index: i32,
    size: u32,
    target: &Path,
) -> Result<(), ApplicationError> {
    if source.metadata().is_ok_and(|metadata| metadata.len() == 0) {
        return Err(std::io::Error::other("Windows application source is empty").into());
    }
    let pixels = nanika_platform::file_icon_pixels(source, icon_index, size)?;
    let pixels = crate::normalize_icon_rgba(&pixels, size, size, size)
        .ok_or_else(|| std::io::Error::other("Windows provided an empty application icon"))?;
    crate::icon_cache::write_png(target, size, size, &pixels)
}

fn read_shell_link(
    state: &mut DiscoveryState,
    path: &Path,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    let Some(link) = load_shell_link(path)? else {
        return Ok(None);
    };
    let target = expand_environment(&link.target);
    let target = match target.canonicalize() {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let Some((executable_length, executable_modified)) = state.windows_executable_stamp(&target)?
    else {
        return Ok(None);
    };
    let working_directory = effective_working_directory(&target, link.working_directory.as_deref());
    let arguments = ApplicationArguments::from_windows_raw(link.arguments);
    let arguments_json = arguments.to_json()?;
    let target_key = path_key(&target);
    // Identity describes the application target, not each discovery source.
    // The selected shortcut still owns its original native activation behavior.
    let identity = stable_hash(&["windows", &target_key, &arguments_json]);
    let display_name = display_name(path);
    let normalized_name = normalize_name(&display_name);
    let icon_resource = link
        .icon_source
        .as_deref()
        .map(expand_environment)
        .filter(|source| source.is_file())
        .unwrap_or_else(|| target.clone());
    let icon_key = (|| -> Result<String, ApplicationError> {
        let resource_stamp = state.metadata(&icon_resource)?;
        let resource_key = crate::icon_cache::key_from_stamp(
            &icon_resource,
            link.icon_index,
            resource_stamp.len(),
            timestamp_nanos(resource_stamp.modified()?),
        );
        let shortcut_stamp = state.metadata(path)?;
        let shortcut_key = crate::icon_cache::key_from_stamp(
            path,
            link.icon_index,
            shortcut_stamp.len(),
            timestamp_nanos(shortcut_stamp.modified()?),
        );
        let target_key_for_icon =
            crate::icon_cache::key_from_stamp(&target, 0, executable_length, executable_modified);
        Ok(stable_hash(&[
            &shortcut_key,
            &resource_key,
            &target_key_for_icon,
        ]))
    })()
    .unwrap_or_else(|error| {
        // Icon metadata cannot invalidate an already validated application.
        eprintln!(
            "application icon key failed for {}: {error}",
            path.display()
        );
        crate::IconCache::fallback_key().to_owned()
    });
    Ok(Some(ApplicationEntry {
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens: normalized_name,
        search_readings: Vec::new(),
        launch_kind: "windows-shell-link".to_owned(),
        target_path: path.to_string_lossy().into_owned(),
        working_directory: working_directory.map(|path| path.to_string_lossy().into_owned()),
        arguments_json,
        bundle_id: None,
        icon_key,
        // The Shell item resolves the shortcut's actual icon resource and index;
        // asking for the DLL itself can return its generic file-type icon.
        icon_source: Some(path.to_path_buf()),
        icon_index: 0,
        priority,
    }))
}

fn read_executable(
    state: &mut DiscoveryState,
    path: &Path,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    let target = path.canonicalize()?;
    let Some((executable_length, executable_modified)) = state.windows_executable_stamp(&target)?
    else {
        return Ok(None);
    };
    let target_key = path_key(&target);
    let working_directory = effective_working_directory(&target, None);
    let arguments_json = ApplicationArguments::empty().to_json()?;
    let identity = stable_hash(&["windows", &target_key, &arguments_json]);
    let display_name = display_name(path);
    let normalized_name = normalize_name(&display_name);
    Ok(Some(ApplicationEntry {
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens: normalized_name,
        search_readings: Vec::new(),
        launch_kind: "executable".to_owned(),
        target_path: target.to_string_lossy().into_owned(),
        working_directory: working_directory.map(|path| path.to_string_lossy().into_owned()),
        arguments_json,
        bundle_id: None,
        icon_key: crate::icon_cache::key_from_stamp(
            &target,
            0,
            executable_length,
            executable_modified,
        ),
        icon_source: Some(target),
        icon_index: 0,
        priority,
    }))
}

fn known_folder(id: &windows_sys::core::GUID) -> Result<PathBuf, ApplicationError> {
    let mut value = std::ptr::null_mut();
    // Resolving a configured OS path must not require a user-owned directory to
    // exist. The scanner distinguishes deletion from inaccessible sources.
    let result = unsafe {
        SHGetKnownFolderPath(
            id,
            KF_FLAG_DONT_VERIFY as u32,
            std::ptr::null_mut::<c_void>() as HANDLE,
            &mut value,
        )
    };
    if result < 0 || value.is_null() {
        return Err(ApplicationError::Io(std::io::Error::from_raw_os_error(
            result,
        )));
    }
    let length = unsafe {
        let mut length = 0;
        while *value.add(length) != 0 {
            length += 1;
        }
        length
    };
    let path = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
        std::slice::from_raw_parts(value, length)
    }));
    unsafe {
        CoTaskMemFree(value.cast());
    }
    Ok(path)
}

fn display_name(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn effective_working_directory(target: &Path, value: Option<&str>) -> Option<PathBuf> {
    let directory = value
        .filter(|value| !value.trim().is_empty())
        .map(expand_environment)
        .or_else(|| target.parent().map(Path::to_path_buf))?;
    Some(directory.canonicalize().unwrap_or(directory))
}

fn load_shell_link(path: &Path) -> Result<Option<ShellLinkMetadata>, ApplicationError> {
    let initialization = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let should_uninitialize = initialization.is_ok();
    if initialization.is_err() && initialization != RPC_E_CHANGED_MODE {
        return Err(windows_error(windows::core::Error::from(initialization)));
    }
    let result = load_shell_link_initialized(path);
    if should_uninitialize {
        unsafe {
            CoUninitialize();
        }
    }
    result
}

fn load_shell_link_initialized(path: &Path) -> Result<Option<ShellLinkMetadata>, ApplicationError> {
    let shell_link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
            .map_err(windows_error)?;
    let persistence: IPersistFile = shell_link.cast().map_err(windows_error)?;
    let link_path = wide_null(path.as_os_str());
    unsafe {
        persistence
            .Load(PCWSTR(link_path.as_ptr()), STGM_READ)
            .map_err(windows_error)?;
    }
    let mut target = vec![0_u16; 32_768];
    unsafe {
        shell_link
            .GetPath(&mut target, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .map_err(windows_error)?;
    }
    let Some(target) = wide_string(&target) else {
        return Ok(None);
    };
    let mut arguments = vec![0_u16; 32_768];
    unsafe {
        shell_link
            .GetArguments(&mut arguments)
            .map_err(windows_error)?;
    }
    let mut working_directory = vec![0_u16; 32_768];
    unsafe {
        shell_link
            .GetWorkingDirectory(&mut working_directory)
            .map_err(windows_error)?;
    }
    let mut icon_source = vec![0_u16; 32_768];
    let mut icon_index = 0_i32;
    unsafe {
        shell_link
            .GetIconLocation(&mut icon_source, &mut icon_index)
            .map_err(windows_error)?;
    }
    Ok(Some(ShellLinkMetadata {
        target,
        arguments: wide_string(&arguments),
        working_directory: wide_string(&working_directory),
        icon_source: wide_string(&icon_source),
        icon_index,
    }))
}

fn wide_string(value: &[u16]) -> Option<String> {
    let length = value
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(value.len());
    (length > 0).then(|| String::from_utf16_lossy(&value[..length]))
}

fn windows_error(error: windows::core::Error) -> ApplicationError {
    ApplicationError::Io(std::io::Error::other(error.to_string()))
}

fn expand_environment(value: &str) -> PathBuf {
    let source = wide_null(std::ffi::OsStr::new(value));
    let required = unsafe { ExpandEnvironmentStringsW(source.as_ptr(), std::ptr::null_mut(), 0) };
    if required == 0 {
        return PathBuf::from(value);
    }
    let mut expanded = vec![0_u16; required as usize];
    let written =
        unsafe { ExpandEnvironmentStringsW(source.as_ptr(), expanded.as_mut_ptr(), required) };
    if written == 0 || written > required {
        return PathBuf::from(value);
    }
    let length = expanded
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(expanded.len());
    PathBuf::from(std::ffi::OsString::from_wide(&expanded[..length]))
}

fn wide_null(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value.encode_wide().chain(std::iter::once(0)).collect()
}
