use nanika_protocol::SystemAction;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_SUCCESS, GetLastError},
    Security::{
        AdjustTokenPrivileges, ImpersonateSelf, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        SecurityImpersonation, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    },
    System::{
        Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        Power::SetSuspendState,
        Shutdown::{
            EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, ExitWindowsEx, LockWorkStation,
            SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
        },
        Threading::{GetCurrentThread, OpenThreadToken},
    },
    UI::{
        Shell::{
            SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, SHERB_NOCONFIRMATION,
            SHEmptyRecycleBinW, ShellExecuteExW,
        },
        WindowsAndMessaging::{
            HWND_BROADCAST, SC_MONITORPOWER, SW_SHOWNORMAL, SendNotifyMessageW, WM_SYSCOMMAND,
        },
    },
};

pub(crate) fn executor() -> std::io::Result<impl Fn(SystemAction) -> Result<(), String> + Send> {
    Ok(_execute)
}

fn _execute(action: SystemAction) -> Result<(), String> {
    match action {
        SystemAction::Lock => _bool(unsafe { LockWorkStation() } != 0, "Lock screen"),
        SystemAction::Sleep | SystemAction::Restart | SystemAction::ShutDown => {
            // Impersonation and shutdown privilege belong only to this short-lived thread.
            // Thread exit releases the token even on failure, without mutating process privileges.
            std::thread::Builder::new()
                .name("nanika-power-request".to_owned())
                .spawn(move || {
                    _enable_shutdown_privilege()?;
                    _power(action)
                })
                .map_err(|error| format!("Could not start power request: {error}"))?
                .join()
                .map_err(|_| "Power request thread failed.".to_owned())?
        }
        SystemAction::TurnOffDisplays => _bool(
            unsafe {
                SendNotifyMessageW(HWND_BROADCAST, WM_SYSCOMMAND, SC_MONITORPOWER as usize, 2)
            } != 0,
            "Turn off displays",
        ),
        SystemAction::LogOut => _bool(unsafe { ExitWindowsEx(EWX_LOGOFF, 0) } != 0, "Log out"),
        SystemAction::OpenTrash | SystemAction::EmptyTrash => {
            _hresult(
                unsafe { CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32) },
                "Initialize Shell",
            )?;
            let result = if action == SystemAction::OpenTrash {
                _open_trash()
            } else {
                _empty_trash()
            };
            unsafe { CoUninitialize() };
            result
        }
    }
}

fn _enable_shutdown_privilege() -> Result<(), String> {
    _bool(
        unsafe { ImpersonateSelf(SecurityImpersonation) } != 0,
        "Create power request token",
    )?;
    let mut token = null_mut();
    _bool(
        unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_ADJUST_PRIVILEGES, 0, &mut token) } != 0,
        "Open power request token",
    )?;
    let result = (|| {
        let mut privileges: TOKEN_PRIVILEGES = unsafe { std::mem::zeroed() };
        privileges.PrivilegeCount = 1;
        let name = windows_sys::w!("SeShutdownPrivilege");
        _bool(
            unsafe { LookupPrivilegeValueW(null(), name, &mut privileges.Privileges[0].Luid) } != 0,
            "Resolve shutdown privilege",
        )?;
        privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        _bool(
            unsafe { AdjustTokenPrivileges(token, 0, &privileges, 0, null_mut(), null_mut()) } != 0,
            "Enable shutdown privilege",
        )?;
        let error = unsafe { GetLastError() };
        if error != ERROR_SUCCESS {
            return Err(format!(
                "Shutdown privilege is unavailable: {}",
                std::io::Error::from_raw_os_error(error as i32)
            ));
        }
        Ok(())
    })();
    unsafe { CloseHandle(token) };
    result
}

fn _power(action: SystemAction) -> Result<(), String> {
    match action {
        SystemAction::Sleep => _bool(unsafe { SetSuspendState(false, false, false) }, "Sleep"),
        SystemAction::Restart | SystemAction::ShutDown => {
            let flags = if action == SystemAction::Restart {
                EWX_REBOOT
            } else {
                EWX_POWEROFF
            };
            _bool(
                unsafe {
                    ExitWindowsEx(flags, SHTDN_REASON_FLAG_PLANNED | SHTDN_REASON_MAJOR_OTHER)
                } != 0,
                "End system session",
            )
        }
        _ => unreachable!("only power operations enter the privileged thread"),
    }
}

fn _open_trash() -> Result<(), String> {
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC;
    info.lpFile = windows_sys::w!("shell:RecycleBinFolder");
    info.nShow = SW_SHOWNORMAL;
    _bool(
        unsafe { ShellExecuteExW(&mut info) } != 0,
        "Open Recycle Bin",
    )
}

fn _empty_trash() -> Result<(), String> {
    _hresult(
        unsafe { SHEmptyRecycleBinW(null_mut(), null(), SHERB_NOCONFIRMATION) },
        "Empty Recycle Bin",
    )
}

fn _bool(success: bool, operation: &str) -> Result<(), String> {
    if success {
        Ok(())
    } else {
        Err(format!(
            "{operation} failed: {}",
            std::io::Error::last_os_error()
        ))
    }
}

fn _hresult(result: i32, operation: &str) -> Result<(), String> {
    if result < 0 {
        Err(format!(
            "{operation} failed: {}",
            windows::core::Error::from_hresult(windows::core::HRESULT(result))
        ))
    } else {
        Ok(())
    }
}
