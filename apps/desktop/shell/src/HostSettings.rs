use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use nanika_config::{ConfigStore, LauncherPreferences};
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub(crate) struct HostSettings {
    store: ConfigStore,
    preferences: Mutex<LauncherPreferences>,
    pub(crate) recording: AtomicBool,
    pub(crate) hide_on_blur: AtomicBool,
    startup: Mutex<Option<nanika_platform::StartupService>>,
}

impl HostSettings {
    pub(crate) fn open(paths: &nanika_storage::NanikaPaths) -> Result<Self, String> {
        let store = ConfigStore::open(paths.app_data_root(), paths.config_root())
            .map_err(|error| error.to_string())?;
        let preferences = LauncherPreferences::load(&store, crate::DEFAULT_HOTKEY)?;
        parse(&preferences.launcher_shortcut)?;
        let hide_on_blur = AtomicBool::new(preferences.hide_on_blur);
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let startup = nanika_platform::StartupService::spawn(executable)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            store,
            preferences: Mutex::new(preferences),
            recording: AtomicBool::new(false),
            hide_on_blur,
            startup: Mutex::new(Some(startup)),
        })
    }

    pub(crate) fn current(&self) -> LauncherPreferences {
        self.preferences
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn save(
        &self,
        app: &tauri::AppHandle,
        updated: LauncherPreferences,
    ) -> Result<LauncherPreferences, String> {
        updated.validate()?;
        let next = parse(&updated.launcher_shortcut)?;
        let mut current = self
            .preferences
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let previous = parse(&current.launcher_shortcut)?;
        let shortcut_changed = next != previous;
        // Reserve the new chord before removing the old one. A conflict leaves
        // the launcher reachable through its existing shortcut.
        if shortcut_changed {
            register(app, &updated.launcher_shortcut)?;
        }
        if let Err(error) = updated.save(&self.store) {
            if !shortcut_changed {
                return Err(error);
            }
            return match app.global_shortcut().unregister(next) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!(
                    "{error}; releasing the new shortcut also failed: {cleanup}"
                )),
            };
        }
        if shortcut_changed && let Err(error) = app.global_shortcut().unregister(previous) {
            let rollback = current.save(&self.store);
            let cleanup = app.global_shortcut().unregister(next);
            let mut message = format!("The previous shortcut could not be released: {error}");
            if let Err(error) = rollback {
                message.push_str(&format!("; restoring preferences also failed: {error}"));
            }
            if let Err(error) = cleanup {
                message.push_str(&format!(
                    "; releasing the new shortcut also failed: {error}"
                ));
            }
            return Err(message);
        }
        app.set_theme(native_theme(updated.theme));
        self.hide_on_blur
            .store(updated.hide_on_blur, Ordering::Release);
        *current = updated.clone();
        Ok(updated)
    }

    pub(crate) fn startup(&self, enabled: Option<bool>) -> Result<&'static str, String> {
        let startup = self
            .startup
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let startup = startup
            .as_ref()
            .ok_or_else(|| "Startup service is shutting down.".to_owned())?;
        let reply = match enabled {
            Some(enabled) => startup.set_enabled(enabled),
            None => startup.query(),
        }
        .map_err(|error| error.to_string())?;
        let status = reply
            .recv()
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        Ok(match status {
            nanika_platform::StartupStatus::Disabled => "disabled",
            nanika_platform::StartupStatus::Enabled => "enabled",
            nanika_platform::StartupStatus::RequiresApproval => "requiresApproval",
            nanika_platform::StartupStatus::NeedsRepair => "needsRepair",
            nanika_platform::StartupStatus::NotFound => "notFound",
        })
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
}

fn parse(value: &str) -> Result<Shortcut, String> {
    if value.len() > 128 {
        return Err("Shortcut is too long.".to_owned());
    }
    let shortcut: Shortcut = value
        .parse()
        .map_err(|error| format!("Invalid shortcut: {error}"))?;
    use tauri_plugin_global_shortcut::Modifiers;
    if !shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER)
    {
        return Err("Use Ctrl, Alt, or Command/Windows with a key.".to_owned());
    }
    Ok(shortcut)
}

pub(crate) fn register(app: &tauri::AppHandle, value: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(parse(value)?, |app, shortcut, event| {
            if event.state == ShortcutState::Pressed {
                // The OS consumes the registered chord before the WebView sees it.
                // Deliver it to the recorder instead of toggling the launcher.
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
        .map_err(|error| format!("Shortcut could not be registered: {error}"))
}

pub(crate) fn native_theme(theme: nanika_config::ThemePreference) -> Option<tauri::Theme> {
    match theme {
        nanika_config::ThemePreference::System => None,
        nanika_config::ThemePreference::Light => Some(tauri::Theme::Light),
        nanika_config::ThemePreference::Dark => Some(tauri::Theme::Dark),
    }
}
