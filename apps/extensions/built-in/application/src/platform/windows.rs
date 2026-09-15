#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{RPC_E_CHANGED_MODE, SIZE};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, IShellLinkW, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
    SIIGBF_RESIZETOFIT, SLGP_RAWPATH, ShellLink,
};
use windows::core::{Interface, PCWSTR};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows_sys::Win32::UI::Shell::{
    ExtractIconExW, FOLDERID_CommonPrograms, FOLDERID_Programs, SHFILEINFOW, SHGFI_ICON,
    SHGFI_LARGEICON, SHGetFileInfoW, SHGetKnownFolderPath,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx};

use super::shell_link_metadata::ShellLinkMetadata;
use crate::normalization::{normalize_name, path_key, stable_hash};
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
    seen_at: u64,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("lnk") {
        read_shell_link(state, path, seen_at, priority)
    } else if extension.eq_ignore_ascii_case("exe") {
        read_executable(state, path, seen_at, priority)
    } else if is_application_bundle(path) {
        read_packaged_application(path, seen_at, priority)
    } else {
        Ok(None)
    }
}

fn read_packaged_application(
    package_root: &Path,
    seen_at: u64,
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
        launch_kind: "windows-packaged".to_owned(),
        target_path: "explorer.exe".to_owned(),
        working_directory: None,
        arguments_json,
        bundle_id: Some(package_id),
        icon_key,
        file_identity: path_key(package_root),
        last_seen_at: seen_at,
        stale: false,
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

pub(super) fn extract_icon(
    source: &Path,
    _icon_index: i32,
    size: u32,
    target: &Path,
) -> Result<(), ApplicationError> {
    if source.metadata().is_ok_and(|metadata| metadata.len() == 0) {
        return Err(ApplicationError::Io(std::io::Error::other(
            "Windows application source is empty",
        )));
    }
    let source = wide_null(source.as_os_str());
    let apartment = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let should_uninitialize = apartment.is_ok();
    let shell_result = unsafe {
        SHCreateItemFromParsingName::<_, _, IShellItemImageFactory>(PCWSTR(source.as_ptr()), None)
    };
    if let Ok(factory) = shell_result {
        let bitmap = unsafe {
            factory.GetImage(
                SIZE {
                    cx: size as i32,
                    cy: size as i32,
                },
                SIIGBF_ICONONLY | SIIGBF_RESIZETOFIT,
            )
        };
        if let Ok(bitmap) = bitmap {
            let result = hbitmap_pixels(bitmap.0, size).and_then(|pixels| {
                let pixels =
                    crate::normalize_icon_rgba(&pixels, size, size, size).ok_or_else(|| {
                        std::io::Error::other("Windows provided an empty application icon")
                    })?;
                crate::icon_cache::write_png(target, size, size, &pixels)
            });
            unsafe {
                DeleteObject(bitmap.0);
            }
            if should_uninitialize {
                unsafe {
                    CoUninitialize();
                }
            }
            if result.is_ok() {
                return result;
            }
        }
    }
    if should_uninitialize {
        unsafe {
            CoUninitialize();
        }
    }
    let mut info = unsafe { std::mem::zeroed::<SHFILEINFOW>() };
    let extracted = unsafe {
        SHGetFileInfoW(
            source.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    let icon = info.hIcon;
    if extracted == 0 || icon.is_null() {
        let mut fallback = std::ptr::null_mut();
        let count = unsafe {
            ExtractIconExW(
                source.as_ptr(),
                _icon_index,
                &mut fallback,
                std::ptr::null_mut(),
                1,
            )
        };
        if count == 0 || fallback.is_null() {
            return Err(ApplicationError::Io(std::io::Error::other(
                "Windows Shell did not provide an application icon",
            )));
        }
        let result = icon_pixels(fallback, size).and_then(|pixels| {
            let pixels =
                crate::normalize_icon_rgba(&pixels, size, size, size).ok_or_else(|| {
                    std::io::Error::other("Windows provided an empty application icon")
                })?;
            crate::icon_cache::write_png(target, size, size, &pixels)
        });
        unsafe {
            DestroyIcon(fallback);
        }
        return result;
    }
    let result = icon_pixels(icon, size).and_then(|pixels| {
        let pixels = crate::normalize_icon_rgba(&pixels, size, size, size)
            .ok_or_else(|| std::io::Error::other("Windows provided an empty application icon"))?;
        crate::icon_cache::write_png(target, size, size, &pixels)
    });
    unsafe {
        DestroyIcon(icon);
    }
    result
}

fn read_shell_link(
    state: &mut DiscoveryState,
    path: &Path,
    seen_at: u64,
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
    let working_directory_key = working_directory
        .as_deref()
        .map_or_else(String::new, path_key);
    let arguments = ApplicationArguments::from_windows_raw(link.arguments);
    let arguments_json = arguments.to_json()?;
    let target_key = path_key(&target);
    let identity = stable_hash(&[
        "windows",
        &target_key,
        &working_directory_key,
        &arguments_json,
    ]);
    let display_name = display_name(path);
    let normalized_name = normalize_name(&display_name);
    let icon_source = link
        .icon_source
        .as_deref()
        .map(expand_environment)
        .filter(|source| source.is_file())
        .unwrap_or_else(|| target.clone());
    let icon_key = crate::icon_cache::key_from_stamp(
        &icon_source,
        link.icon_index,
        executable_length,
        executable_modified,
    );
    Ok(Some(ApplicationEntry {
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens: normalized_name,
        launch_kind: "windows-shell-link".to_owned(),
        target_path: target.to_string_lossy().into_owned(),
        working_directory: working_directory.map(|path| path.to_string_lossy().into_owned()),
        arguments_json,
        bundle_id: None,
        icon_key,
        file_identity: target_key,
        last_seen_at: seen_at,
        stale: false,
        icon_source: Some(icon_source),
        icon_index: link.icon_index,
        priority,
    }))
}

fn read_executable(
    state: &mut DiscoveryState,
    path: &Path,
    seen_at: u64,
    priority: usize,
) -> Result<Option<ApplicationEntry>, ApplicationError> {
    let target = path.canonicalize()?;
    let Some((executable_length, executable_modified)) = state.windows_executable_stamp(&target)?
    else {
        return Ok(None);
    };
    let target_key = path_key(&target);
    let working_directory = effective_working_directory(&target, None);
    let working_directory_key = working_directory
        .as_deref()
        .map_or_else(String::new, path_key);
    let arguments_json = ApplicationArguments::empty().to_json()?;
    let identity = stable_hash(&[
        "windows",
        &target_key,
        &working_directory_key,
        &arguments_json,
    ]);
    let display_name = display_name(path);
    let normalized_name = normalize_name(&display_name);
    Ok(Some(ApplicationEntry {
        entry_id: format!("app.{identity}"),
        source_key: path_key(path),
        display_name,
        normalized_name: normalized_name.clone(),
        normalized_tokens: normalized_name,
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
        file_identity: target_key,
        last_seen_at: seen_at,
        stale: false,
        icon_source: Some(target),
        icon_index: 0,
        priority,
    }))
}

fn known_folder(id: &windows_sys::core::GUID) -> Result<PathBuf, ApplicationError> {
    let mut value = std::ptr::null_mut();
    let result = unsafe {
        SHGetKnownFolderPath(id, 0, std::ptr::null_mut::<c_void>() as HANDLE, &mut value)
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

fn icon_pixels(
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    size: u32,
) -> Result<Vec<u8>, ApplicationError> {
    let mut pixels = draw_icon_bgra(icon, size, 0)?;
    let has_alpha = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
    if has_alpha {
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            if pixel[3] > 0 && pixel[3] < u8::MAX {
                let alpha = u16::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = ((u16::from(*channel) * 255) / alpha).min(255) as u8;
                }
            }
        }
        return Ok(pixels);
    }
    let white = draw_icon_bgra(icon, size, u8::MAX)?;
    Ok(crate::windows_alpha_recovery::recover_rgba(pixels, &white))
}

fn hbitmap_pixels(
    bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    size: u32,
) -> Result<Vec<u8>, ApplicationError> {
    let mut info = unsafe { std::mem::zeroed::<BITMAPINFO>() };
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: size as i32,
        biHeight: -(size as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { std::mem::zeroed() }
    };
    let mut pixels = vec![0_u8; (size * size * 4) as usize];
    let screen = unsafe { GetDC(std::ptr::null_mut()) };
    if screen.is_null() {
        return Err(ApplicationError::Io(std::io::Error::last_os_error()));
    }
    let copied = unsafe {
        GetDIBits(
            screen,
            bitmap,
            0,
            size,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    unsafe {
        ReleaseDC(std::ptr::null_mut(), screen);
    }
    if copied == 0 {
        return Err(ApplicationError::Io(std::io::Error::last_os_error()));
    }
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Ok(pixels)
}

fn draw_icon_bgra(
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    size: u32,
    background: u8,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bitmap = unsafe { std::mem::zeroed::<BITMAPINFO>() };
    bitmap.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: size as i32,
        biHeight: -(size as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { std::mem::zeroed() }
    };
    let screen = unsafe { GetDC(std::ptr::null_mut()) };
    if screen.is_null() {
        return Err(ApplicationError::Io(std::io::Error::last_os_error()));
    }
    let memory = unsafe { CreateCompatibleDC(screen) };
    if memory.is_null() {
        unsafe {
            ReleaseDC(std::ptr::null_mut(), screen);
        }
        return Err(ApplicationError::Io(std::io::Error::last_os_error()));
    }
    let mut bits = std::ptr::null_mut();
    let dib = unsafe {
        CreateDIBSection(
            screen,
            &bitmap,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        )
    };
    if dib.is_null() || bits.is_null() {
        unsafe {
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);
        }
        return Err(ApplicationError::Io(std::io::Error::last_os_error()));
    }
    let previous = unsafe { SelectObject(memory, dib) };
    let pixel_count = (size * size) as usize;
    let buffer = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), pixel_count * 4) };
    for pixel in buffer.as_chunks_mut::<4>().0 {
        *pixel = [background, background, background, 0];
    }
    let drawn = unsafe {
        DrawIconEx(
            memory,
            0,
            0,
            icon,
            size as i32,
            size as i32,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        )
    };
    let pixels = if drawn == 0 {
        Vec::new()
    } else {
        buffer.to_vec()
    };
    unsafe {
        SelectObject(memory, previous);
        DeleteObject(dib);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
    if drawn == 0 {
        return Err(ApplicationError::Io(std::io::Error::last_os_error()));
    }
    Ok(pixels)
}

fn wide_null(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value.encode_wide().chain(std::iter::once(0)).collect()
}
