use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf, Prefix};

#[cfg(test)]
#[path = "../../../tests/adapters/windows/reveal.rs"]
mod tests;

use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::{SHOpenFolderAndSelectItems, SHParseDisplayName};
use windows::core::PCWSTR;

pub(crate) fn reveal(path: &Path) -> std::io::Result<()> {
    path.symlink_metadata()?;
    let shell_path = _shell_path(path)?;
    let wide = shell_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let result = (|| {
        let mut item = std::ptr::null_mut();
        unsafe { SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut item, 0, None) }.map_err(
            |error| std::io::Error::other(format!("Could not resolve Shell item: {error}")),
        )?;
        // A fully qualified PIDL with no child list selects the item in its parent.
        let result = unsafe { SHOpenFolderAndSelectItems(item, None, 0) };
        unsafe { CoTaskMemFree(Some(item.cast())) };
        result
            .map_err(|error| std::io::Error::other(format!("Could not select Shell item: {error}")))
    })();
    unsafe { CoUninitialize() };
    result
}

// SHParseDisplayName rejects the verbatim prefixes returned by canonicalize.
// Keep filesystem identities unchanged; only the Shell boundary uses DOS/UNC spelling.
fn _shell_path(path: &Path) -> std::io::Result<PathBuf> {
    let mut parts = path.components();
    let Some(Component::Prefix(prefix)) = parts.next() else {
        return Err(std::io::Error::other(
            "Shell location must be an absolute path",
        ));
    };
    let mut result = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:\\", drive as char)),
        Prefix::VerbatimUNC(server, share) => {
            let mut value = std::ffi::OsString::from(r"\\");
            value.push(server);
            value.push(r"\");
            value.push(share);
            PathBuf::from(value)
        }
        Prefix::Disk(_) | Prefix::UNC(_, _) => return Ok(path.to_path_buf()),
        _ => {
            return Err(std::io::Error::other(
                "Unsupported Shell location namespace",
            ));
        }
    };
    for part in parts {
        if !matches!(part, Component::RootDir) {
            result.push(part.as_os_str());
        }
    }
    Ok(result)
}
