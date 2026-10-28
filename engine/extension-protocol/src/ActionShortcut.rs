use serde::{Deserialize, Serialize};

/// A root-search action chord. Primary is Command on macOS and Control on Windows.
/// The key explicitly selects virtual-key or physical-code semantics.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ActionShortcut {
    pub key: crate::ShortcutKey,
    pub modifiers: Vec<crate::ShortcutModifier>
}

impl ActionShortcut {
    pub fn primary(key: impl Into<String>) -> Self {
        Self {
            key: crate::ShortcutKey::Virtual(key.into()),
            modifiers: vec![crate::ShortcutModifier::Primary]
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        use crate::ShortcutModifier::Primary;
        let modifiers: std::collections::HashSet<_> = self.modifiers.iter().collect();
        if !self.key.is_valid()
            || !modifiers.contains(&Primary)
            || modifiers.len() != self.modifiers.len()
        {
            return Err(
                "action shortcut requires a primary modifier, unique modifiers and a supported key"
                    .into()
            );
        }
        Ok(())
    }

    /// Normalize modifier order for conflict checks.
    pub(crate) fn identity(&self) -> (&crate::ShortcutKey, bool, bool) {
        (
            &self.key,
            self.modifiers.contains(&crate::ShortcutModifier::Alt),
            self.modifiers.contains(&crate::ShortcutModifier::Shift)
        )
    }
}
