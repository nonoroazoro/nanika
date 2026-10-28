use serde::{Deserialize, Serialize};

/// Virtual key identity and physical UI Events code are distinct protocol values.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ShortcutKey {
    /// Virtual key identity, also used as its menu label (for example E or 1).
    /// Independent of the physical position and of Shift/Alt character output.
    Virtual(String),
    /// UI Events physical code. Its menu label follows the native primary-modifier layout.
    Physical(String)
}

impl ShortcutKey {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Virtual(key) => {
                key == "Enter"
                    || (key.len() == 1
                        && (key.as_bytes()[0].is_ascii_uppercase()
                            || key.as_bytes()[0].is_ascii_digit()))
            }
            Self::Physical(key) => {
                let bytes = key.as_bytes();
                key == "Enter"
                    || key == "NumpadEnter"
                    || (bytes.len() == 4
                        && bytes.starts_with(b"Key")
                        && bytes[3].is_ascii_uppercase())
                    || (bytes.len() == 6
                        && bytes.starts_with(b"Digit")
                        && bytes[5].is_ascii_digit())
            }
        }
    }
}
