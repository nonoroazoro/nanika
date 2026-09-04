pub(crate) fn configure_extension_command(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};

        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    }

    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;

        command.process_group(0);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = command;
}
