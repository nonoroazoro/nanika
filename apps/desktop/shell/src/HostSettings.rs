use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering}
};

use nanika_config::{ConfigStore, LauncherPreferences};
use nanika_host::{OperationError, OperationErrorCode as Code};
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub(crate) struct HostSettings {
    store: ConfigStore,
    preferences: Mutex<crate::HostPreferencesState>,
    pub(crate) recording: AtomicBool,
    pub(crate) hide_on_blur: AtomicBool,
    startup: Mutex<Option<nanika_platform::StartupService>>
}

impl HostSettings {
    pub(crate) fn open(paths: &nanika_storage::NanikaPaths) -> Result<Self, String> {
        let store = ConfigStore::open(paths.app_data_root(), paths.config_root())
            .map_err(|error| error.to_string())?;
        let mut preferences = LauncherPreferences::load(&store, crate::DEFAULT_HOTKEY)?;
        if let Err(error) = parse(&preferences.launcher_shortcut) {
            tracing::warn!(%error, "invalid launcher shortcut; using default");
            preferences.launcher_shortcut = crate::DEFAULT_HOTKEY.to_owned();
        }
        let hide_on_blur = AtomicBool::new(preferences.hide_on_blur);
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let startup = nanika_platform::StartupService::spawn(executable)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            store,
            preferences: Mutex::new(crate::HostPreferencesState {
                localization: nanika_localization::LocalizationSnapshot::new(
                    preferences.language,
                    nanika_platform::preferred_ui_languages().as_deref()
                ),
                preferences
            }),
            recording: AtomicBool::new(false),
            hide_on_blur,
            startup: Mutex::new(Some(startup))
        })
    }

    pub(crate) fn current(&self) -> LauncherPreferences {
        self.snapshot().preferences
    }

    pub(crate) fn snapshot(&self) -> crate::HostPreferencesState {
        self.preferences
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn save(
        &self,
        app: &tauri::AppHandle,
        request: crate::HostSettingsChange
    ) -> Result<crate::SettingsWriteResult<LauncherPreferences>, crate::HostSettingsFailure> {
        let mut current = self
            .preferences
            .try_lock()
            .map_err(|_| OperationError::new(Code::OperationPending))?;
        let mut updated = current.preferences.clone();
        match request {
            crate::HostSettingsChange::LauncherShortcut(value) => updated.launcher_shortcut = value,
            crate::HostSettingsChange::Theme(value) => updated.theme = value,
            crate::HostSettingsChange::Language(value) => updated.language = value,
            crate::HostSettingsChange::HideOnBlur(value) => updated.hide_on_blur = value
        }
        let languages = (updated.language != current.preferences.language
            && updated.language == nanika_localization::LanguagePreference::System)
            .then(nanika_platform::preferred_ui_languages)
            .flatten();
        let localization = current
            .localization
            .updated(updated.language, languages.as_deref())
            .map_err(|detail| OperationError::with_detail(Code::LanguageUpdateFailed, detail))?;
        let result = self._save(app, updated, &mut current.preferences);
        if result.is_ok() && localization != current.localization {
            current.localization = localization;
            // Publication stays inside the transaction lock so concurrent writes cannot reorder it.
            app.state::<crate::DesktopState>()
                .publish_localization(localization);
        }
        let saved =
            LauncherPreferences::load(&self.store, crate::DEFAULT_HOTKEY).map_err(|error| {
                crate::HostSettingsFailure::confirmation_failed(result.as_ref().err(), error)
            })?;
        let mut outcome = crate::SettingsWriteResult {
            values: current.preferences.clone(),
            saved,
            effective: result.as_ref().ok().cloned(),
            error: result.err()
        };
        let localization = current.localization;
        drop(current);
        // Native menu calls may wait for the event loop. Never hold the preference transaction lock.
        if outcome.error.is_none()
            && let Err(error) = app
                .state::<crate::NativeLocalization>()
                .apply(app, localization)
        {
            outcome.error =
                Some(OperationError::with_detail(Code::NativeLanguageUpdateFailed, error).into());
        }
        Ok(outcome)
    }

    pub(crate) fn refresh_language(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let localization = {
            let mut current = self
                .preferences
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if current.preferences.language == nanika_localization::LanguagePreference::System {
                let languages = nanika_platform::preferred_ui_languages();
                if languages.is_none() {
                    tracing::warn!("preferred UI languages unavailable; retaining current locale");
                }
                let updated = current
                    .localization
                    .updated(current.preferences.language, languages.as_deref())?;
                if updated != current.localization {
                    current.localization = updated;
                    app.state::<crate::DesktopState>()
                        .publish_localization(updated);
                }
            }
            current.localization
        };
        app.state::<crate::NativeLocalization>()
            .apply(app, localization)
    }

    pub(crate) fn startup(
        &self,
        enabled: Option<bool>
    ) -> Result<nanika_platform::StartupStatus, OperationError> {
        let startup = self
            .startup
            .try_lock()
            .map_err(|_| OperationError::new(Code::OperationPending))?;
        let startup = startup
            .as_ref()
            .ok_or_else(|| OperationError::new(Code::ApplicationShuttingDown))?;
        let code = if enabled.is_some() {
            Code::StartupWriteUnconfirmed
        } else {
            Code::StartupReadFailed
        };
        let reply = match enabled {
            Some(enabled) => startup.set_enabled(enabled),
            None => startup.query()
        }
        .map_err(|error| OperationError::with_detail(code, error.to_string()))?;
        let status = reply
            .recv()
            .map_err(|error| OperationError::with_detail(code, error.to_string()))?
            .map_err(|error| OperationError::with_detail(code, error.to_string()))?;
        Ok(status)
    }

    pub(crate) fn shutdown(&self) {
        if let Some(startup) = self
            .startup
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            startup.shutdown();
        }
    }
    fn _save(
        &self,
        app: &tauri::AppHandle,
        updated: LauncherPreferences,
        current: &mut LauncherPreferences
    ) -> Result<LauncherPreferences, crate::HostSettingsFailure> {
        _persist_preferences(
            &updated,
            current,
            |preferences| preferences.save(&self.store),
            |shortcut| register(app, shortcut),
            |shortcut| {
                app.global_shortcut()
                    .unregister(shortcut)
                    .map_err(|error| error.to_string())
            }
        )?;
        // Unrelated preference writes must not repaint every native window.
        if updated.theme != current.theme {
            app.set_theme(native_theme(updated.theme));
        }
        self.hide_on_blur
            .store(updated.hide_on_blur, Ordering::Release);
        *current = updated.clone();
        Ok(updated)
    }
}

fn parse(value: &str) -> Result<Shortcut, OperationError> {
    if value.len() > 128 {
        return Err(OperationError::new(Code::InvalidShortcut));
    }
    let shortcut: Shortcut = value
        .parse()
        .map_err(|error| OperationError::with_detail(Code::InvalidShortcut, format!("{error}")))?;
    use tauri_plugin_global_shortcut::Modifiers;
    if !shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return Err(OperationError::new(Code::ShortcutModifiersRequired));
    }
    Ok(shortcut)
}

pub(crate) fn register(app: &tauri::AppHandle, value: &str) -> Result<(), OperationError> {
    app.global_shortcut()
        .on_shortcut(parse(value)?, |app, shortcut, event| {
            if event.state == ShortcutState::Pressed {
                // The OS intercepts this chord; forward it to the recorder without toggling the launcher.
                if app
                    .state::<HostSettings>()
                    .recording
                    .load(Ordering::Acquire)
                {
                    app.state::<crate::DesktopState>().shortcut_recorded();
                    return;
                }
                if let Some(delay) = nanika_platform::current_hotkey_delivery_delay(shortcut.id()) {
                    tracing::debug!(delay_ms = delay.as_millis(), "global shortcut delivered");
                }
                if let Err(error) = crate::toggle_launcher(app) {
                    tracing::error!(%error, "global shortcut could not toggle launcher");
                }
            }
        })
        .map_err(|error| {
            OperationError::with_detail(Code::ShortcutRegistrationFailed, error.to_string())
        })
}

pub(crate) fn native_theme(theme: nanika_config::ThemePreference) -> Option<tauri::Theme> {
    match theme {
        nanika_config::ThemePreference::System => None,
        nanika_config::ThemePreference::Light => Some(tauri::Theme::Light),
        nanika_config::ThemePreference::Dark => Some(tauri::Theme::Dark)
    }
}

// Registration and persistence remain ordered here; callers own locks and publication.
fn _persist_preferences(
    updated: &LauncherPreferences,
    current: &LauncherPreferences,
    mut persist: impl FnMut(&LauncherPreferences) -> Result<(), String>,
    register: impl FnOnce(&str) -> Result<(), OperationError>,
    mut unregister: impl FnMut(Shortcut) -> Result<(), String>
) -> Result<(), crate::HostSettingsFailure> {
    updated
        .validate()
        .map_err(|detail| OperationError::with_detail(Code::InvalidHostPreferences, detail))?;
    let next = parse(&updated.launcher_shortcut)?;
    let previous = parse(&current.launcher_shortcut)?;
    let changed = next != previous;
    // Reserve the new chord before writing so a conflict preserves the old shortcut.
    if changed {
        register(&updated.launcher_shortcut)?;
    }
    if let Err(detail) = persist(updated) {
        let mut failure = crate::HostSettingsFailure::from(OperationError::with_detail(
            Code::PreferencesSaveFailed,
            detail
        ));
        if changed {
            failure.cleanup = unregister(next).err().map(|detail| {
                Box::new(OperationError::with_detail(
                    Code::ShortcutCleanupFailed,
                    detail
                ))
            });
        }
        return Err(failure);
    }
    if changed && let Err(detail) = unregister(previous) {
        let mut failure = crate::HostSettingsFailure::from(OperationError::with_detail(
            Code::ShortcutReleaseFailed,
            detail
        ));
        failure.rollback = persist(current).err().map(|detail| {
            Box::new(OperationError::with_detail(
                Code::PreferencesRollbackFailed,
                detail
            ))
        });
        failure.cleanup = unregister(next).err().map(|detail| {
            Box::new(OperationError::with_detail(
                Code::ShortcutCleanupFailed,
                detail
            ))
        });
        return Err(failure);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/HostSettings.rs"]
mod tests;
