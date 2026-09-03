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
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows_sys::Win32::UI::Shell::{
    FOLDERID_CommonPrograms, FOLDERID_Programs, SHGetKnownFolderPath,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DI_NORMAL, DestroyIcon, DrawIconEx, PrivateExtractIconsW,
};

use super::shell_link_metadata::ShellLinkMetadata;
use crate::normalization::{normalize_name, path_key, stable_hash};
use crate::{ApplicationArguments, ApplicationEntry, ApplicationError, DiscoveryState};

pub(super) fn standard_roots() -> Result<Vec<PathBuf>, ApplicationError> {
    [FOLDERID_Programs, FOLDERID_CommonPrograms]
        .iter()
        .map(known_folder)
        .collect()
}

pub(super) fn is_application_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("lnk") || extension.eq_ignore_ascii_case("exe")
        })
}

pub(super) fn is_application_bundle(_path: &Path) -> bool {
    false
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
    } else {
        Ok(None)
    }
}

pub(super) fn extract_icon(
    source: &Path,
    icon_index: i32,
    size: u32,
    target: &Path,
) -> Result<(), ApplicationError> {
    let source = wide_null(source.as_os_str());
    let mut icon = std::ptr::null_mut();
    let extracted = unsafe {
        PrivateExtractIconsW(
            source.as_ptr(),
            icon_index,
            size as i32,
            size as i32,
            &mut icon,
            std::ptr::null_mut(),
            1,
            0,
        )
    };
    if extracted == 0 || icon.is_null() {
        return Err(ApplicationError::Io(std::io::Error::other(
            "Windows did not provide an application icon",
        )));
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
    let icon_key = if icon_source == target {
        crate::icon_cache::key_from_stamp(
            &icon_source,
            link.icon_index,
            executable_length,
            executable_modified,
        )
    } else {
        String::new()
    };
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
    Ok(crate::legacy_icon::recover_legacy_rgba(pixels, &white))
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
