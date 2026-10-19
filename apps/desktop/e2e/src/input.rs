#[cfg(target_os = "macos")]
#[path = "input/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "input/windows.rs"]
mod platform;

pub(crate) fn press(key: &str) -> Result<Vec<u16>, String> {
    platform::press(key)
}

pub(crate) fn wait_for_release(keys: &[u16]) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while let Some(key) = keys.iter().find(|key| platform::is_down(**key)) {
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "Native key {key} did not return to the released state"
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}
