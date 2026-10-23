use std::sync::Mutex;

use nanika_localization::LocalizationSnapshot;
use tauri::Manager;

/// Stable native menu items retain their identity while only their presentation changes.
pub(crate) struct NativeLocalization {
    _open: tauri::menu::MenuItem<tauri::Wry>,
    _quit: tauri::menu::MenuItem<tauri::Wry>,
    _applied: Mutex<crate::native_localization_state::NativeLocalizationState>
}

impl NativeLocalization {
    pub(crate) fn new(
        open: tauri::menu::MenuItem<tauri::Wry>,
        quit: tauri::menu::MenuItem<tauri::Wry>
    ) -> Self {
        Self {
            _open: open,
            _quit: quit,
            _applied: Mutex::new(Default::default())
        }
    }

    #[cfg(feature = "e2e")]
    pub(crate) fn labels(&self) -> tauri::Result<[String; 2]> {
        Ok([self._open.text()?, self._quit.text()?])
    }

    pub(crate) fn apply(
        &self,
        app: &tauri::AppHandle,
        snapshot: LocalizationSnapshot
    ) -> Result<(), String> {
        let mut applied = self
            ._applied
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        applied.apply(snapshot, || {
            let resources = crate::host_resources::resources()?;
            let label = |id| {
                resources.format(
                    snapshot.locale,
                    &nanika_protocol::message(id, []),
                    nanika_platform::target_platform().as_str(),
                    128,
                    false
                )
            };
            // Prepare all text before touching native objects; an invalid resource cannot half-apply.
            let open = label("tray-open")?;
            let quit = label("tray-quit")?;
            let title = label("settings-window-title")?;
            self._open
                .set_text(open)
                .map_err(|error| error.to_string())?;
            self._quit
                .set_text(quit)
                .map_err(|error| error.to_string())?;
            if let Some(settings) = app.get_webview_window("settings") {
                settings
                    .set_title(&title)
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }
}
