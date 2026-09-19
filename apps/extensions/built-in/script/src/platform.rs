use nanika_protocol::{LaunchArguments, LaunchDescriptor};
use std::path::Path;

#[cfg(target_os = "windows")]
#[path = "platform/windows.rs"]
mod adapter;
#[cfg(target_os = "macos")]
#[path = "platform/macos.rs"]
mod adapter;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
compile_error!("Script discovery supports Windows 10+ and macOS 13+ only");

pub(crate) fn launch_descriptor(path: &Path) -> Result<LaunchDescriptor, String> {
    if !path.is_file() {
        return Err(format!("script is unavailable: {}", path.display()));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let program = adapter::interpreter(&extension).ok_or("unsupported script type")?;
    let mut values = if extension == "ps1" {
        vec![
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-File".to_owned(),
        ]
    } else {
        Vec::new()
    };
    values.push(path.to_str().ok_or("script path is not UTF-8")?.to_owned());
    Ok(LaunchDescriptor::Program {
        program: program.to_owned(),
        arguments: LaunchArguments::Structured { values },
        working_directory: path
            .parent()
            .and_then(|parent| parent.to_str())
            .map(str::to_owned),
    })
}
