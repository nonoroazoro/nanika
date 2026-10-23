use nanika_localization::{LocalizationSnapshot, SupportedLocale};

/// Native APIs can fail after a partial update. The latest requested revision and
/// fully applied locale are separate facts; the caller serializes native mutations.
#[derive(Default)]
pub(crate) struct NativeLocalizationState {
    _revision: u64,
    _locale: Option<SupportedLocale>
}

impl NativeLocalizationState {
    pub(crate) fn apply(
        &mut self,
        snapshot: LocalizationSnapshot,
        update: impl FnOnce() -> Result<(), String>
    ) -> Result<(), String> {
        if snapshot.revision < self._revision {
            return Ok(());
        }
        self._revision = snapshot.revision;
        if self._locale == Some(snapshot.locale) {
            return Ok(());
        }
        self._locale = None;
        update()?;
        self._locale = Some(snapshot.locale);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/NativeLocalizationState.rs"]
mod tests;
