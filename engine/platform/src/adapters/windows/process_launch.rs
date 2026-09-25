use std::process::Command;

pub(crate) fn windows_application(value: &str) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_FLAG_NO_UI, SEE_MASK_INVOKEIDLIST, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
        ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let path = Path::new(value);
    if value.contains('\0')
        || !path.is_absolute()
        || !path.extension().is_some_and(|extension| {
            ["lnk", "exe", "com"]
                .iter()
                .any(|allowed| extension.eq_ignore_ascii_case(allowed))
        })
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Windows application must be an absolute .lnk, .exe, or .com path without NUL characters",
        ));
    }
    if !path.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Windows application is not a file",
        ));
    }
    let path = std::path::absolute(path)?;
    // Executables use the discovered parent directory; Shell Links retain their activation settings.
    let directory = if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        None
    } else {
        path.parent().map(|parent| {
            parent
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        })
    };
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let initialized = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if initialized < 0 {
        return Err(std::io::Error::other(format!(
            "could not initialize Windows Shell COM: HRESULT {initialized:#010x}"
        )));
    }
    let mut launch = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // Without a message loop, wait for Shell acceptance only; Windows owns security prompts.
        fMask: SEE_MASK_INVOKEIDLIST | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpFile: path.as_ptr(),
        lpDirectory: directory
            .as_ref()
            .map_or(std::ptr::null(), |value| value.as_ptr()),
        nShow: SW_SHOWNORMAL,
        ..unsafe { std::mem::zeroed() }
    };
    let result = if unsafe { ShellExecuteExW(&mut launch) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    };
    unsafe { CoUninitialize() };
    result
}

pub(crate) fn apply_windows_raw(command: &mut Command, value: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;

    command.raw_arg(value);
    Ok(())
}

pub(crate) fn shell_command(value: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let interpreter = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(interpreter);
    command.args(["/d", "/s", "/c"]);
    command.raw_arg(format!("\"{value}\""));
    command
}

pub(crate) fn mac_application(_bundle_path: &str) -> std::io::Result<Command> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "macOS application launch is unsupported on this platform",
    ))
}
