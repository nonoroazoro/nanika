use clipboard_win::{Clipboard, raw};

pub(crate) struct ClipboardSnapshot {
    _formats: Vec<(u32, Vec<u8>)>
}

impl ClipboardSnapshot {
    pub(crate) fn capture() -> Result<Self, String> {
        let _clipboard = Clipboard::new_attempts(10).map_err(|error| error.to_string())?;
        let mut formats = Vec::new();
        let mut bytes = 0_usize;
        for format in raw::EnumFormats::new() {
            // GDI handles and owner-display formats cannot be restored as HGLOBAL bytes.
            if !matches!(format, 1 | 7 | 8 | 13 | 15 | 16 | 17 | 0xC000..=0xFFFF) {
                return Err(format!(
                    "Clipboard format {format} cannot be safely snapshotted; clipboard was not changed."
                ));
            }
            let mut data = Vec::new();
            raw::get_vec(format, &mut data).map_err(|error| error.to_string())?;
            bytes = bytes.saturating_add(data.len());
            if bytes > super::MAX_SNAPSHOT_BYTES {
                return Err("Clipboard snapshot exceeds 64 MiB; clipboard was not changed.".into());
            }
            formats.push((format, data));
        }
        Ok(Self { _formats: formats })
    }

    pub(crate) fn restore(self) -> Result<(), String> {
        let _clipboard = Clipboard::new_attempts(10).map_err(|error| error.to_string())?;
        raw::empty().map_err(|error| error.to_string())?;
        for (format, data) in self._formats {
            raw::set_without_clear(format, &data).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
