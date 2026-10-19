use std::io::{BufRead, Write};

use clipboard_rs::{Clipboard, ClipboardContext};

#[cfg(target_os = "macos")]
#[path = "clipboard/macos/ClipboardSnapshot.rs"]
mod snapshot;
#[cfg(target_os = "windows")]
#[path = "clipboard/windows/ClipboardSnapshot.rs"]
mod snapshot;

pub(crate) const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn run(value: &str) -> Result<(), String> {
    let original = snapshot::ClipboardSnapshot::capture()?;
    let clipboard = ClipboardContext::new().map_err(|error| error.to_string())?;
    let replacement = format!("Nanika clipboard replacement {}", std::process::id());
    if let Err(error) = clipboard.set_text(value.to_owned()) {
        original.restore()?;
        return Err(error.to_string());
    }
    let result = _exchange(&clipboard, value, &replacement);
    // Do not overwrite content copied by the user while the test was running.
    if !clipboard
        .get_text()
        .is_ok_and(|text| text == value || text == replacement)
    {
        return Err("Clipboard changed outside the fixture; preserved the newer content.".into());
    }
    original.restore()?;
    result
}

fn _exchange(clipboard: &ClipboardContext, value: &str, replacement: &str) -> Result<(), String> {
    _reply("seeded")?;
    for line in std::io::stdin().lock().lines() {
        match line.map_err(|error| error.to_string())?.as_str() {
            "replace" => {
                clipboard
                    .set_text(replacement.to_owned())
                    .map_err(|error| error.to_string())?;
                _reply("replaced")?;
            }
            "verify" => {
                if clipboard.get_text().map_err(|error| error.to_string())? != value {
                    return Err("The application did not copy the selected clipboard entry.".into());
                }
                _reply("verified")?;
            }
            _ => return Err("Unknown clipboard fixture command".into())
        }
    }
    Ok(())
}

fn _reply(message: &str) -> Result<(), String> {
    println!("{message}");
    std::io::stdout().flush().map_err(|error| error.to_string())
}
