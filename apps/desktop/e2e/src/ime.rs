use std::io::{Read, Write};

#[cfg(target_os = "macos")]
#[path = "ime/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "ime/windows.rs"]
mod platform;

pub(crate) fn run(pid: u32) -> Result<(), String> {
    platform::run(pid)
}

fn _wait_for_owner() -> Result<(), String> {
    println!("selected");
    std::io::stdout()
        .flush()
        .map_err(|error| error.to_string())?;
    let mut byte = [0];
    if std::io::stdin()
        .read(&mut byte)
        .map_err(|error| error.to_string())?
        != 0
    {
        return Err("Expected EOF to restore the input source".into());
    }
    Ok(())
}
