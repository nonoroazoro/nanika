use std::mem::size_of;

use windows_sys::Win32::Foundation::{GetLastError, POINT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::{OverlayPosition, PlatformError, centered_position};

pub(crate) fn active_overlay_position(
    width_points: f32,
    height_points: f32,
    _current_scale_factor: f32,
) -> Result<OverlayPosition, PlatformError> {
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return Err(PlatformError::OsCode {
            operation: "GetCursorPos",
            code: unsafe { GetLastError() },
        });
    }
    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return Err(PlatformError::Message("MonitorFromPoint failed".to_owned()));
    }
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return Err(PlatformError::OsCode {
            operation: "GetMonitorInfoW",
            code: unsafe { GetLastError() },
        });
    }
    let mut dpi_x = 96;
    let mut dpi_y = 96;
    if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } < 0 {
        return Err(PlatformError::Message("GetDpiForMonitor failed".to_owned()));
    }
    let work = info.rcWork;
    Ok(centered_position(
        f64::from(work.left),
        f64::from(work.top),
        f64::from(work.right),
        f64::from(work.bottom),
        f64::from(width_points) * f64::from(dpi_x) / 96.0,
        f64::from(height_points) * f64::from(dpi_y) / 96.0,
    ))
}
