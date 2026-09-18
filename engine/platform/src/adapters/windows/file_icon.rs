use std::os::windows::fs::MetadataExt;
use std::path::Path;
use windows::Win32::Foundation::{RPC_E_CHANGED_MODE, SIZE};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Controls::{IImageList, ILD_TRANSPARENT};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SHGetImageList, SHIL_JUMBO, SIIGBF,
    SIIGBF_ICONONLY, SIIGBF_RESIZETOFIT,
};
use windows::core::PCWSTR;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, SelectObject,
};
use windows_sys::Win32::UI::Shell::{
    ExtractIconExW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SYSICONINDEX, SHGetFileInfoW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx};

pub(crate) fn stamp(metadata: &std::fs::Metadata) -> String {
    format!(
        "{}:{}:{}:{}",
        metadata.creation_time(),
        metadata.last_write_time(),
        metadata.file_size(),
        metadata.file_attributes()
    )
}

pub(crate) fn shell_pixels(path: &Path, size: u32) -> std::io::Result<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    let source = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
        return Err(std::io::Error::other(
            windows::core::Error::from(initialized).to_string(),
        ));
    }
    let _apartment = Apartment(initialized.is_ok());

    // Explorer prefers a content thumbnail for supported files and falls back
    // to their icon. This keeps image collections distinguishable while text,
    // executables, shortcuts, and directories retain their native Shell icon.
    if let Ok(pixels) = image_factory_pixels(&source, size, SIIGBF_RESIZETOFIT) {
        return Ok(pixels);
    }

    // SHGetFileInfo resolves the icon index that Explorer assigns to this
    // concrete file or directory. SHIL_JUMBO supplies its 256 px native icon
    // when the item has no thumbnail or thumbnail acquisition fails.
    let mut info = unsafe { std::mem::zeroed::<SHFILEINFOW>() };
    let resolved = unsafe {
        SHGetFileInfoW(
            source.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };
    if resolved != 0
        && let Ok(images) = unsafe { SHGetImageList::<IImageList>(SHIL_JUMBO as i32) }
        && let Ok(icon) = unsafe { images.GetIcon(info.iIcon, ILD_TRANSPARENT.0) }
    {
        let result = icon_pixels(icon.0, size).and_then(visible_pixels);
        unsafe {
            DestroyIcon(icon.0);
        }
        if result.is_ok() {
            return result;
        }
    }

    shell_file_info_pixels(&source, size)
}

pub(crate) fn list_pixels(path: &Path, size: u32) -> std::io::Result<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
        return Err(std::io::Error::other(
            windows::core::Error::from(initialized).to_string(),
        ));
    }
    let _apartment = Apartment(initialized.is_ok());
    let source = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // Preserve high-resolution artwork before reducing it for high-DPI list rows.
    // The image list supplies an HICON without a thumbnail frame. Normalize at
    // its native 256 px size so legacy icons on a larger transparent canvas fit too.
    let mut info = unsafe { std::mem::zeroed::<SHFILEINFOW>() };
    let resolved = unsafe {
        SHGetFileInfoW(
            source.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };
    if resolved != 0
        && let Ok(images) = unsafe { SHGetImageList::<IImageList>(SHIL_JUMBO as i32) }
        && let Ok(icon) = unsafe { images.GetIcon(info.iIcon, ILD_TRANSPARENT.0) }
    {
        let result = icon_pixels(icon.0, 256).and_then(visible_pixels);
        unsafe {
            DestroyIcon(icon.0);
        }
        if let Ok(pixels) = result
            && let Some(normalized) = crate::normalize_icon_rgba(&pixels, 256, 256, size)
        {
            return Ok(normalized);
        }
    }
    let pixels = shell_file_info_pixels(&source, size)?;
    crate::normalize_icon_rgba(&pixels, size, size, size)
        .ok_or_else(|| std::io::Error::other("Windows provided an empty list icon"))
}

pub(crate) fn pixels(path: &Path, icon_index: i32, size: u32) -> std::io::Result<Vec<u8>> {
    use std::os::windows::ffi::OsStrExt;
    let source = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
        return Err(std::io::Error::other(
            windows::core::Error::from(initialized).to_string(),
        ));
    }
    // Drop all COM interfaces before balancing this thread's initialization.
    let _apartment = Apartment(initialized.is_ok());
    if let Ok(pixels) = image_factory_pixels(&source, size, SIIGBF_ICONONLY | SIIGBF_RESIZETOFIT) {
        return Ok(pixels);
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
    let mut icon = info.hIcon;
    if extracted == 0 || icon.is_null() {
        if !icon.is_null() {
            unsafe {
                DestroyIcon(icon);
            }
        }
        icon = std::ptr::null_mut();
        let count = unsafe {
            ExtractIconExW(
                source.as_ptr(),
                icon_index,
                &mut icon,
                std::ptr::null_mut(),
                1,
            )
        };
        if count == 0 || icon.is_null() {
            return Err(std::io::Error::other(
                "Windows Shell did not provide a file icon",
            ));
        }
    }
    let result = icon_pixels(icon, size).and_then(visible_pixels);
    unsafe {
        DestroyIcon(icon);
    }
    result
}

fn image_factory_pixels(source: &[u16], size: u32, flags: SIIGBF) -> std::io::Result<Vec<u8>> {
    let factory = unsafe {
        SHCreateItemFromParsingName::<_, _, IShellItemImageFactory>(PCWSTR(source.as_ptr()), None)
    }
    .map_err(|error| std::io::Error::other(error.to_string()))?;
    let bitmap = unsafe {
        factory.GetImage(
            SIZE {
                cx: size as i32,
                cy: size as i32,
            },
            flags,
        )
    }
    .map_err(|error| std::io::Error::other(error.to_string()))?;
    let result = hbitmap_pixels(bitmap.0, size).and_then(visible_pixels);
    unsafe {
        DeleteObject(bitmap.0);
    }
    result
}

fn shell_file_info_pixels(source: &[u16], size: u32) -> std::io::Result<Vec<u8>> {
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
    if extracted == 0 || info.hIcon.is_null() {
        if !info.hIcon.is_null() {
            unsafe {
                DestroyIcon(info.hIcon);
            }
        }
        return Err(std::io::Error::other(
            "Windows Shell did not provide a file icon",
        ));
    }
    let result = icon_pixels(info.hIcon, size).and_then(visible_pixels);
    unsafe {
        DestroyIcon(info.hIcon);
    }
    result
}

fn visible_pixels(pixels: Vec<u8>) -> std::io::Result<Vec<u8>> {
    if pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0) {
        Ok(pixels)
    } else {
        Err(std::io::Error::other("Windows provided an empty file icon"))
    }
}

struct Apartment(bool);
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

fn icon_pixels(
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    size: u32,
) -> std::io::Result<Vec<u8>> {
    let mut pixels = draw_icon_bgra(icon, size, 0)?;
    let has_alpha = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
    if has_alpha {
        crate::windows_alpha_recovery::unpremultiply_bgra_to_rgba(&mut pixels);
        return Ok(pixels);
    }
    let white = draw_icon_bgra(icon, size, u8::MAX)?;
    Ok(crate::windows_alpha_recovery::recover_rgba(pixels, &white))
}

fn hbitmap_pixels(
    bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    size: u32,
) -> std::io::Result<Vec<u8>> {
    // Shell artwork may be smaller or non-square despite the requested dimensions.
    let mut dimensions = unsafe { std::mem::zeroed::<BITMAP>() };
    let read = unsafe {
        GetObjectW(
            bitmap,
            std::mem::size_of::<BITMAP>() as i32,
            (&mut dimensions as *mut BITMAP).cast(),
        )
    };
    if read == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let width = dimensions.bmWidth;
    let height = dimensions.bmHeight;
    if !(1..=512).contains(&width) || !(1..=512).contains(&height) {
        return Err(std::io::Error::other(
            "Windows returned invalid icon bitmap dimensions",
        ));
    }
    let mut info = unsafe { std::mem::zeroed::<BITMAPINFO>() };
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { std::mem::zeroed() }
    };
    let mut pixels = vec![0_u8; (width * height * 4) as usize];
    let screen = unsafe { GetDC(std::ptr::null_mut()) };
    if screen.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    let copied = unsafe {
        GetDIBits(
            screen,
            bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    let error = std::io::Error::last_os_error();
    unsafe {
        ReleaseDC(std::ptr::null_mut(), screen);
    }
    if copied != height {
        return Err(error);
    }
    // IShellItemImageFactory returns a 32-bit PARGB bitmap. Convert its
    // premultiplied BGRA channels before normalization and PNG encoding.
    crate::windows_alpha_recovery::unpremultiply_bgra_to_rgba(&mut pixels);
    crate::normalize_icon_rgba(&pixels, width as u32, height as u32, size)
        .ok_or_else(|| std::io::Error::other("Windows provided an empty file icon"))
}

fn draw_icon_bgra(
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    size: u32,
    background: u8,
) -> std::io::Result<Vec<u8>> {
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
        return Err(std::io::Error::last_os_error());
    }
    let memory = unsafe { CreateCompatibleDC(screen) };
    if memory.is_null() {
        unsafe {
            ReleaseDC(std::ptr::null_mut(), screen);
        }
        return Err(std::io::Error::last_os_error());
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
        return Err(std::io::Error::last_os_error());
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
        return Err(std::io::Error::last_os_error());
    }
    Ok(pixels)
}
